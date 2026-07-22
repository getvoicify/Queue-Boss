use qb_core::{
    BackendError, BackendInfo, JobDetail, JobFilter, JobId, JobSummary, Page, QueueSummary,
};
use serde::Serialize;
use tokio::task::AbortHandle;

use crate::state::{AppState, ConnectionId};

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
            BackendError::Connection(_) => ("connection", "connection failed"),
            BackendError::Unsupported(_) => ("unsupported", "operation not supported"),
            BackendError::NotFound(_) => ("notFound", "not found"),
            BackendError::Internal(_) => ("internal", "internal error"),
        };
        Self {
            kind: kind.to_owned(),
            message: message.to_owned(),
        }
    }
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

    use qb_core::testing::FakeBackend;
    use qb_core::{Clock, ManualClock, QueueBackend};

    use super::*;
    use crate::state::AppState;

    fn state_with_fake() -> AppState {
        let mut backends: HashMap<String, Arc<dyn QueueBackend>> = HashMap::new();
        backends.insert("sandbox".to_owned(), Arc::new(FakeBackend::new()));
        let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(0));
        AppState::new(backends, clock)
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
}
