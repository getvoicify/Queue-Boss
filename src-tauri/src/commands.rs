use std::sync::Arc;

use qb_core::{
    BackendError, BackendInfo, JobDetail, JobFilter, JobId, JobSummary, Page, QueueBackend,
    QueueSummary,
};
use serde::{Deserialize, Serialize};
use tokio::task::AbortHandle;

use crate::state::{AppState, ConnectionId};

/// Fixed connection id for the single pg-boss connection this app manages.
pub const PGBOSS_CONNECTION_ID: &str = "pgboss";
/// Fixed connection id for the always-present in-memory sandbox backend.
pub const SANDBOX_CONNECTION_ID: &str = "sandbox";

/// Error envelope returned to the webview. The message is always sanitized —
/// a raw driver string must never cross the IPC boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub kind: String,
    pub message: String,
}

impl CommandError {
    pub fn unknown_connection() -> Self {
        Self {
            kind: "notFound".to_owned(),
            message: "unknown connection id".to_owned(),
        }
    }
}

impl From<BackendError> for CommandError {
    fn from(err: BackendError) -> Self {
        let (kind, message) = match err {
            BackendError::Connection(_) => {
                ("connection".to_owned(), "connection failed".to_owned())
            }
            BackendError::Unsupported(msg) => ("unsupported".to_owned(), msg),
            BackendError::NotFound(_) => ("notFound".to_owned(), "not found".to_owned()),
            BackendError::Internal(_) => ("internal".to_owned(), "internal error".to_owned()),
        };
        Self { kind, message }
    }
}

/// Deserializable pg-boss connection config. Accepts either a single
/// `connectionString` or the discrete connection parts; serde picks the shape.
#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum PgConnectConfig {
    ConnectionString {
        #[serde(rename = "connectionString")]
        connection_string: String,
    },
    Parts {
        host: String,
        port: u16,
        database: String,
        user: String,
        password: String,
        #[serde(rename = "sslMode")]
        ssl_mode: String,
        schema: Option<String>,
    },
}

impl std::fmt::Debug for PgConnectConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PgConnectConfig::ConnectionString { .. } => f
                .debug_struct("ConnectionString")
                .field("connection_string", &"<redacted>")
                .finish(),
            PgConnectConfig::Parts {
                host,
                port,
                database,
                user,
                ssl_mode,
                schema,
                ..
            } => f
                .debug_struct("Parts")
                .field("host", host)
                .field("port", port)
                .field("database", database)
                .field("user", user)
                .field("password", &"<redacted>")
                .field("ssl_mode", ssl_mode)
                .field("schema", schema)
                .finish(),
        }
    }
}

impl PgConnectConfig {
    /// The secret to persist: the whole connection string, or the password.
    pub(crate) fn credential(&self) -> &str {
        match self {
            PgConnectConfig::ConnectionString { connection_string } => connection_string,
            PgConnectConfig::Parts { password, .. } => password,
        }
    }

    /// The pg-boss schema to read; defaults to `pgboss` when unspecified.
    pub(crate) fn schema(&self) -> &str {
        match self {
            PgConnectConfig::ConnectionString { .. } => "pgboss",
            PgConnectConfig::Parts { schema, .. } => schema.as_deref().unwrap_or("pgboss"),
        }
    }
}

/// Connect the single pg-boss backend. `build` owns pool construction so this
/// stays testable without a live database. Order is load-bearing: build →
/// test_connection → persist credential → register. A failure at any step
/// registers nothing, and a build/test failure persists no credential. The
/// credential value never reaches an error or log path.
pub async fn connect_impl<F>(
    state: &AppState,
    config: &PgConnectConfig,
    build: F,
) -> Result<ConnectionId, CommandError>
where
    F: FnOnce(&PgConnectConfig) -> Result<Arc<dyn QueueBackend>, BackendError>,
{
    let backend = build(config).map_err(CommandError::from)?;
    backend
        .test_connection()
        .await
        .map_err(CommandError::from)?;
    state
        .secrets
        .set(PGBOSS_CONNECTION_ID, config.credential())
        .map_err(|_| CommandError {
            kind: "internal".to_owned(),
            message: "internal error".to_owned(),
        })?;
    state.register(PGBOSS_CONNECTION_ID.to_owned(), backend);
    state.abort_task(PGBOSS_CONNECTION_ID);
    Ok(PGBOSS_CONNECTION_ID.to_owned())
}

/// Tear a connection down: abort its poll task, drop its backend, and delete
/// its stored credential. Idempotent — each step is no-op-safe. The
/// always-present sandbox connection cannot be disconnected.
pub fn disconnect_impl(state: &AppState, id: &str) -> Result<(), CommandError> {
    if id == SANDBOX_CONNECTION_ID {
        return Err(CommandError {
            kind: "unsupported".to_owned(),
            message: "the sandbox connection cannot be disconnected".to_owned(),
        });
    }
    state.abort_task(id);
    state.remove_backend(id);
    let _ = state.secrets.delete(id);
    Ok(())
}

pub async fn test_connection_impl(
    state: &AppState,
    connection_id: &str,
) -> Result<BackendInfo, CommandError> {
    state
        .backend(connection_id)?
        .test_connection()
        .await
        .map_err(CommandError::from)
}

pub async fn list_queues_impl(
    state: &AppState,
    connection_id: &str,
) -> Result<Vec<QueueSummary>, CommandError> {
    state
        .backend(connection_id)?
        .list_queues()
        .await
        .map_err(CommandError::from)
}

pub async fn list_jobs_impl(
    state: &AppState,
    connection_id: &str,
    filter: JobFilter,
) -> Result<Page<JobSummary>, CommandError> {
    state
        .backend(connection_id)?
        .list_jobs(filter)
        .await
        .map_err(CommandError::from)
}

pub async fn get_job_impl(
    state: &AppState,
    connection_id: &str,
    id: &str,
) -> Result<JobDetail, CommandError> {
    state
        .backend(connection_id)?
        .get_job(&JobId::from(id))
        .await
        .map_err(CommandError::from)
}

/// Runtime-agnostic orchestration for `subscribe_counts`: resolve the backend
/// (an unknown id is a typed `notFound` BEFORE anything is spawned), then run
/// `spawn` to launch the poll task and register its handle (aborting/replacing
/// any prior task for the same connection). `spawn` owns the actual task launch
/// so this stays testable without a Tauri app or async runtime wiring.
pub(crate) fn subscribe_counts_impl<F>(
    state: &AppState,
    connection_id: ConnectionId,
    spawn: F,
) -> Result<(), CommandError>
where
    F: FnOnce() -> AbortHandle,
{
    state.backend(&connection_id)?;
    let handle = spawn();
    state.replace_task(connection_id, handle);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use async_trait::async_trait;
    use qb_core::testing::FakeBackend;
    use qb_core::{Capabilities, Clock, ManualClock, QueueBackend};
    use qb_platform::{InMemorySecretStore, SecretStore, SecretStoreError};

    use super::*;
    use crate::state::AppState;

    struct FailingSecretStore;

    impl SecretStore for FailingSecretStore {
        fn get(&self, _key: &str) -> Result<Option<String>, SecretStoreError> {
            Ok(None)
        }
        fn set(&self, _key: &str, _value: &str) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::Backend)
        }
        fn delete(&self, _key: &str) -> Result<(), SecretStoreError> {
            Ok(())
        }
    }

    fn state_with_fake() -> AppState {
        let mut backends: HashMap<String, Arc<dyn QueueBackend>> = HashMap::new();
        backends.insert("sandbox".to_owned(), Arc::new(FakeBackend::new()));
        let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(0));
        AppState::new(backends, clock, Arc::new(InMemorySecretStore::new()))
    }

    struct UnsupportedBackend;

    #[async_trait]
    impl QueueBackend for UnsupportedBackend {
        async fn test_connection(&self) -> Result<BackendInfo, BackendError> {
            Err(BackendError::Unsupported(
                "pg-boss v10 required (schema versions 21–24); found schema v20".to_owned(),
            ))
        }
        async fn list_queues(&self) -> Result<Vec<QueueSummary>, BackendError> {
            unreachable!()
        }
        async fn list_jobs(&self, _filter: JobFilter) -> Result<Page<JobSummary>, BackendError> {
            unreachable!()
        }
        async fn get_job(&self, _id: &JobId) -> Result<JobDetail, BackendError> {
            unreachable!()
        }
        fn capabilities(&self) -> Capabilities {
            unreachable!()
        }
    }

    #[test]
    fn backend_error_maps_each_variant_to_its_kind() {
        assert_eq!(
            CommandError::from(BackendError::Connection("x".to_owned())).kind,
            "connection"
        );
        assert_eq!(
            CommandError::from(BackendError::Unsupported("x".to_owned())).kind,
            "unsupported"
        );
        assert_eq!(
            CommandError::from(BackendError::NotFound("x".to_owned())).kind,
            "notFound"
        );
        assert_eq!(
            CommandError::from(BackendError::Internal("x".to_owned())).kind,
            "internal"
        );
    }

    #[test]
    fn backend_error_message_is_sanitized() {
        let raw = "postgres://user:s3cr3t@db.internal:5432";
        let err = CommandError::from(BackendError::Connection(raw.to_owned()));
        assert!(
            !err.message.contains("s3cr3t"),
            "leaked secret: {}",
            err.message
        );
        assert!(!err.message.contains(raw), "leaked raw driver string");
    }

    #[tokio::test]
    async fn test_connection_impl_delegates_to_backend() {
        let state = state_with_fake();
        let info = test_connection_impl(&state, "sandbox").await.unwrap();
        assert_eq!(info.name, "fake");
        assert!(info.healthy);
    }

    #[tokio::test]
    async fn list_queues_impl_delegates_and_maps() {
        let state = state_with_fake();
        let queues = list_queues_impl(&state, "sandbox").await.unwrap();
        assert_eq!(queues.len(), 1);
        assert_eq!(queues[0].name, "default");
        assert_eq!(queues[0].total_depth, 3);
    }

    #[tokio::test]
    async fn list_jobs_impl_delegates_to_backend() {
        let state = state_with_fake();
        let page = list_jobs_impl(
            &state,
            "sandbox",
            JobFilter {
                queue: None,
                states: None,
                time_window: None,
                search: None,
                cursor: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn get_job_impl_delegates_and_echoes_id() {
        let state = state_with_fake();
        let detail = get_job_impl(&state, "sandbox", "abc").await.unwrap();
        assert_eq!(detail.summary.id, JobId::from("abc"));
    }

    #[tokio::test]
    async fn unknown_connection_id_is_not_found_not_panic() {
        let state = state_with_fake();
        let err = list_queues_impl(&state, "nope").await.unwrap_err();
        assert_eq!(err.kind, "notFound");
    }

    #[tokio::test]
    async fn subscribe_counts_impl_spawns_and_registers_task_for_known_connection() {
        let state = state_with_fake();
        let task = tokio::spawn(std::future::pending::<()>());
        let handle = task.abort_handle();

        let result = subscribe_counts_impl(&state, "sandbox".to_owned(), move || handle);

        assert!(result.is_ok());
        assert!(
            state.tasks.lock().unwrap().contains_key("sandbox"),
            "the spawned task's handle must be registered"
        );

        task.abort();
    }

    #[tokio::test]
    async fn subscribe_counts_impl_unknown_connection_is_not_found_and_does_not_spawn() {
        let state = state_with_fake();

        let result = subscribe_counts_impl(&state, "nope".to_owned(), || {
            panic!("spawn must not run for an unknown connection")
        });

        assert_eq!(result.unwrap_err().kind, "notFound");
        assert!(
            state.tasks.lock().unwrap().is_empty(),
            "no task should be registered for an unknown connection"
        );
    }

    #[tokio::test]
    async fn connect_impl_registers_backend_and_persists_credential() {
        let state = state_with_fake();
        let config = PgConnectConfig::ConnectionString {
            connection_string: "postgres://u:p@h/db".to_owned(),
        };

        let id = connect_impl(&state, &config, |_| {
            Ok(Arc::new(FakeBackend::new()) as Arc<dyn QueueBackend>)
        })
        .await
        .expect("connect should succeed");

        assert_eq!(id, "pgboss");
        assert!(state.backend("pgboss").is_ok());
        assert_eq!(
            state.secrets.get("pgboss").unwrap(),
            Some("postgres://u:p@h/db".to_owned())
        );
    }

    #[test]
    fn deserializes_connection_string_variant() {
        let config: PgConnectConfig =
            serde_json::from_str(r#"{"connectionString":"postgres://u:p@h/db"}"#).unwrap();
        assert_eq!(config.credential(), "postgres://u:p@h/db");
        assert_eq!(config.schema(), "pgboss");
    }

    #[test]
    fn deserializes_parts_variant_with_and_without_schema() {
        let with_schema: PgConnectConfig = serde_json::from_str(
            r#"{"host":"h","port":5432,"database":"d","user":"u","password":"p","sslMode":"require","schema":"custom"}"#,
        )
        .unwrap();
        assert_eq!(with_schema.credential(), "p");
        assert_eq!(with_schema.schema(), "custom");

        let without_schema: PgConnectConfig = serde_json::from_str(
            r#"{"host":"h","port":5432,"database":"d","user":"u","password":"p","sslMode":"require"}"#,
        )
        .unwrap();
        assert_eq!(without_schema.credential(), "p");
        assert_eq!(without_schema.schema(), "pgboss");
    }

    #[test]
    fn unsupported_message_passes_through() {
        let err = CommandError::from(BackendError::Unsupported("v10 required".to_owned()));
        assert_eq!(err.kind, "unsupported");
        assert_eq!(err.message, "v10 required");
    }

    #[tokio::test]
    async fn connect_impl_unsupported_passes_message_through_and_persists_nothing() {
        let state = state_with_fake();
        let config = PgConnectConfig::ConnectionString {
            connection_string: "postgres://u:p@h/db".to_owned(),
        };

        let err = connect_impl(&state, &config, |_| {
            Ok(Arc::new(UnsupportedBackend) as Arc<dyn QueueBackend>)
        })
        .await
        .expect_err("an unsupported backend must fail connect");

        assert_eq!(
            err.message,
            "pg-boss v10 required (schema versions 21–24); found schema v20"
        );
        assert!(state.backend("pgboss").is_err());
        assert_eq!(state.secrets.get("pgboss").unwrap(), None);
    }

    #[tokio::test]
    async fn connect_impl_build_failure_is_generic_and_persists_nothing() {
        let state = state_with_fake();
        let config = PgConnectConfig::ConnectionString {
            connection_string: "postgres://u:p@h/db".to_owned(),
        };

        let err = connect_impl(&state, &config, |_| {
            Err(BackendError::Connection(
                "postgres://user:s3cr3t@host".to_owned(),
            ))
        })
        .await
        .expect_err("a build failure must fail connect");

        assert_eq!(err.kind, "connection");
        assert!(
            !err.message.contains("s3cr3t"),
            "leaked secret: {}",
            err.message
        );
        assert!(state.backend("pgboss").is_err());
        assert_eq!(state.secrets.get("pgboss").unwrap(), None);
    }

    #[tokio::test]
    async fn disconnect_impl_removes_backend_and_deletes_secret() {
        let state = state_with_fake();
        state.register("pgboss".to_owned(), Arc::new(FakeBackend::new()));
        state.secrets.set("pgboss", "cred").unwrap();
        let t = tokio::spawn(std::future::pending::<()>());
        state.replace_task("pgboss".to_owned(), t.abort_handle());

        disconnect_impl(&state, "pgboss").expect("disconnect should succeed");

        assert!(state.backend("pgboss").is_err());
        assert_eq!(state.secrets.get("pgboss").unwrap(), None);
        assert!(!state.tasks.lock().unwrap().contains_key("pgboss"));
    }

    #[test]
    fn disconnect_impl_refuses_to_disconnect_the_sandbox() {
        let state = state_with_fake();

        let err =
            disconnect_impl(&state, "sandbox").expect_err("sandbox must not be disconnectable");

        assert_eq!(err.kind, "unsupported");
        assert!(err.message.contains("sandbox"), "message: {}", err.message);
        assert!(state.backend("sandbox").is_ok());
    }

    #[test]
    fn disconnect_impl_is_idempotent_for_an_unregistered_id() {
        let state = state_with_fake();
        disconnect_impl(&state, "gone").expect("disconnecting a missing id must not panic");
    }

    #[tokio::test]
    async fn connect_impl_aborts_a_stale_poll_task_on_reconnect() {
        let state = state_with_fake();
        let stale = tokio::spawn(std::future::pending::<()>());
        state.replace_task("pgboss".to_owned(), stale.abort_handle());

        connect_impl(
            &state,
            &PgConnectConfig::ConnectionString {
                connection_string: "postgres://u:p@h/db".to_owned(),
            },
            |_| Ok(Arc::new(FakeBackend::new()) as Arc<dyn QueueBackend>),
        )
        .await
        .unwrap();

        assert!(
            !state.tasks.lock().unwrap().contains_key("pgboss"),
            "the stale poll task must be aborted and removed on reconnect"
        );
    }

    #[tokio::test]
    async fn connect_impl_secret_store_failure_is_internal_and_registers_nothing() {
        let mut backends: HashMap<String, Arc<dyn QueueBackend>> = HashMap::new();
        backends.insert("sandbox".to_owned(), Arc::new(FakeBackend::new()));
        let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(0));
        let state = AppState::new(backends, clock, Arc::new(FailingSecretStore));
        let config = PgConnectConfig::ConnectionString {
            connection_string: "postgres://u:p@h/db".to_owned(),
        };

        let err = connect_impl(&state, &config, |_| {
            Ok(Arc::new(FakeBackend::new()) as Arc<dyn QueueBackend>)
        })
        .await
        .expect_err("a secret-store failure must fail connect");

        assert_eq!(err.kind, "internal");
        assert!(
            state.backend("pgboss").is_err(),
            "nothing must be registered when persisting the credential fails"
        );
    }

    #[test]
    fn internal_and_notfound_errors_are_generic_and_sanitized() {
        let raw = "postgres://user:s3cr3t@db.internal:5432";
        let e = CommandError::from(BackendError::Internal(raw.to_owned()));
        assert_eq!(e.message, "internal error");
        assert!(!e.message.contains("s3cr3t"));
        let e = CommandError::from(BackendError::NotFound(raw.to_owned()));
        assert_eq!(e.message, "not found");
        assert!(!e.message.contains("s3cr3t"));
    }

    #[test]
    fn debug_redacts_credentials() {
        let a = PgConnectConfig::ConnectionString {
            connection_string: "postgres://u:s3cr3t@h/db".to_owned(),
        };
        let a_debug = format!("{a:?}");
        assert!(
            !a_debug.contains("s3cr3t"),
            "connection string leaked in Debug"
        );
        assert!(
            a_debug.contains("<redacted>"),
            "connection string field must render as redacted, not be omitted: {a_debug}"
        );
        assert!(
            a_debug.contains("ConnectionString"),
            "Debug must render the ConnectionString struct name: {a_debug}"
        );
        let b = PgConnectConfig::Parts {
            host: "h".to_owned(),
            port: 5432,
            database: "d".to_owned(),
            user: "u".to_owned(),
            password: "s3cr3t".to_owned(),
            ssl_mode: "require".to_owned(),
            schema: None,
        };
        let b_debug = format!("{b:?}");
        assert!(!b_debug.contains("s3cr3t"), "password leaked in Debug");
        assert!(
            b_debug.contains("<redacted>"),
            "password field must render as redacted, not be omitted: {b_debug}"
        );
        assert!(
            b_debug.contains("Parts"),
            "Debug must render the Parts struct name: {b_debug}"
        );
        assert!(
            b_debug.contains("\"h\""),
            "Debug must render the non-secret host field: {b_debug}"
        );
    }
}
