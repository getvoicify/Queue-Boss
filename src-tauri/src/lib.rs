pub mod commands;
mod counts;
mod poller;
pub mod state;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use qb_backends::{PgBossBackend, SandboxBackend};
use qb_core::{
    BackendError, BackendInfo, Clock, JobDetail, JobFilter, JobSummary, Page, QueueBackend,
    QueueSummary, SystemClock,
};
use qb_platform::{OsSecretStore, SecretStore};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::PgPool;
use tauri::ipc::Channel;
use tauri::State;

use crate::commands::{
    connect_impl, disconnect_impl, get_job_impl, list_jobs_impl, list_queues_impl,
    subscribe_counts_impl, test_connection_impl, CommandError, PgConnectConfig,
};
use crate::counts::QueueCounts;
use crate::poller::{poll_loop, DEFAULT_POLL_INTERVAL_MS};
use crate::state::{AppState, ConnectionId};

/// Seed for the default in-memory sandbox backend registered at startup.
const SANDBOX_SEED: u64 = 0xC0FFEE;

#[tauri::command]
async fn test_connection(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<BackendInfo, CommandError> {
    test_connection_impl(state.inner(), &connection_id).await
}

#[tauri::command]
async fn list_queues(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<QueueSummary>, CommandError> {
    list_queues_impl(state.inner(), &connection_id).await
}

#[tauri::command]
async fn list_jobs(
    connection_id: String,
    filter: JobFilter,
    state: State<'_, AppState>,
) -> Result<Page<JobSummary>, CommandError> {
    list_jobs_impl(state.inner(), &connection_id, filter).await
}

#[tauri::command]
async fn get_job(
    connection_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<JobDetail, CommandError> {
    get_job_impl(state.inner(), &connection_id, &id).await
}

/// Start (or restart) the per-connection counts poll task, streaming snapshots
/// into `channel`. Re-subscribing aborts and replaces the previous task.
#[tauri::command]
async fn subscribe_counts(
    connection_id: String,
    channel: Channel<QueueCounts>,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let backend = state.backend(&connection_id)?;
    let clock = state.clock.clone();
    let poll_conn = connection_id.clone();
    subscribe_counts_impl(state.inner(), connection_id, move || {
        tauri::async_runtime::spawn(poll_loop(
            poll_conn,
            backend,
            clock,
            channel,
            DEFAULT_POLL_INTERVAL_MS,
        ))
        .inner()
        .abort_handle()
    })
}

/// Build a lazy pg-boss connection pool from a config. Synchronous (no I/O), so
/// nothing is held across a lock. Credentials never appear in a returned error.
fn build_pool(config: &PgConnectConfig) -> Result<PgPool, BackendError> {
    match config {
        PgConnectConfig::ConnectionString { connection_string } => {
            PgPool::connect_lazy(connection_string)
                .map_err(|_| BackendError::Connection("invalid connection string".to_owned()))
        }
        PgConnectConfig::Parts {
            host,
            port,
            database,
            user,
            password,
            ssl_mode,
            ..
        } => {
            let ssl = PgSslMode::from_str(ssl_mode)
                .map_err(|_| BackendError::Connection("invalid sslMode".to_owned()))?;
            let opts = PgConnectOptions::new()
                .host(host)
                .port(*port)
                .database(database)
                .username(user)
                .password(password)
                .ssl_mode(ssl);
            Ok(PgPoolOptions::new().connect_lazy_with(opts))
        }
    }
}

fn build_pgboss_backend(config: &PgConnectConfig) -> Result<Arc<dyn QueueBackend>, BackendError> {
    let pool = build_pool(config)?;
    Ok(Arc::new(PgBossBackend::with_schema(pool, config.schema())) as Arc<dyn QueueBackend>)
}

#[tauri::command]
async fn connect_pgboss(
    config: PgConnectConfig,
    state: State<'_, AppState>,
) -> Result<ConnectionId, CommandError> {
    connect_impl(state.inner(), &config, build_pgboss_backend).await
}

#[tauri::command]
async fn disconnect(connection_id: String, state: State<'_, AppState>) -> Result<(), CommandError> {
    disconnect_impl(state.inner(), &connection_id)
}

/// Assemble the Tauri-managed application state with the default backends.
fn build_app_state() -> AppState {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let mut backends: HashMap<ConnectionId, Arc<dyn QueueBackend>> = HashMap::new();
    backends.insert(
        "sandbox".to_owned(),
        Arc::new(SandboxBackend::new(clock.clone(), SANDBOX_SEED)) as Arc<dyn QueueBackend>,
    );
    let secrets: Arc<dyn SecretStore> = Arc::new(OsSecretStore::new("queue-boss"));
    AppState::new(backends, clock, secrets)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(build_app_state())
        .invoke_handler(tauri::generate_handler![
            test_connection,
            list_queues,
            list_jobs,
            get_job,
            subscribe_counts,
            connect_pgboss,
            disconnect
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_state_registers_a_resolvable_healthy_sandbox() {
        let state = build_app_state();
        let backend = state
            .backend("sandbox")
            .expect("default state must register the sandbox backend");
        let info = backend
            .test_connection()
            .await
            .expect("sandbox backend must report healthy");
        assert_eq!(info.name, "sandbox");
        assert!(info.healthy);
    }

    #[tokio::test]
    async fn build_pool_accepts_a_connection_string() {
        let config = PgConnectConfig::ConnectionString {
            connection_string: "postgres://u:p@localhost/db".to_owned(),
        };
        assert!(build_pool(&config).is_ok());
    }

    #[test]
    fn build_pool_rejects_an_invalid_ssl_mode() {
        let config = PgConnectConfig::Parts {
            host: "localhost".to_owned(),
            port: 5432,
            database: "db".to_owned(),
            user: "u".to_owned(),
            password: "s3cr3t".to_owned(),
            ssl_mode: "bogus".to_owned(),
            schema: None,
        };
        let err = build_pool(&config).expect_err("an invalid sslMode must fail");
        assert!(matches!(err, BackendError::Connection(_)));
        let BackendError::Connection(message) = &err else {
            unreachable!()
        };
        assert!(!message.contains("s3cr3t"), "leaked credential: {message}");
    }
}
