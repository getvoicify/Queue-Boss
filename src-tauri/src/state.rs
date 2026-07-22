use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use qb_core::{Clock, QueueBackend};
use tokio::task::AbortHandle;

use crate::commands::CommandError;

pub type ConnectionId = String;

/// Tauri-managed application state: the resolvable backends, the wall clock the
/// poller stamps snapshots with, and the live poll tasks keyed by connection.
pub struct AppState {
    pub backends: HashMap<ConnectionId, Arc<dyn QueueBackend>>,
    pub clock: Arc<dyn Clock>,
    pub tasks: Mutex<HashMap<ConnectionId, AbortHandle>>,
}

impl AppState {
    pub fn new(
        backends: HashMap<ConnectionId, Arc<dyn QueueBackend>>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            backends,
            clock,
            tasks: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve the backend for `id`; an unknown id is a typed `notFound`.
    pub fn backend(&self, id: &str) -> Result<Arc<dyn QueueBackend>, CommandError> {
        self.backends
            .get(id)
            .cloned()
            .ok_or_else(CommandError::unknown_connection)
    }

    /// Register a poll task for `id`, aborting and replacing any existing one.
    /// The mutex is never held across an `.await` — every operation here is sync.
    pub fn replace_task(&self, id: ConnectionId, handle: AbortHandle) {
        let mut tasks = self.tasks.lock().expect("tasks mutex poisoned");
        if let Some(previous) = tasks.insert(id, handle) {
            previous.abort();
        }
    }

    /// Abort and drop the poll task for `id`, if any (connection removal).
    #[allow(dead_code)] // wired to a disconnect command in a later child.
    pub fn abort_task(&self, id: &str) {
        let mut tasks = self.tasks.lock().expect("tasks mutex poisoned");
        if let Some(handle) = tasks.remove(id) {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future;

    use qb_core::testing::FakeBackend;
    use qb_core::ManualClock;

    use super::*;

    fn empty_state() -> AppState {
        AppState::new(HashMap::new(), Arc::new(ManualClock::new(0)))
    }

    fn state_with_fake() -> AppState {
        let mut backends: HashMap<ConnectionId, Arc<dyn QueueBackend>> = HashMap::new();
        backends.insert("sandbox".to_owned(), Arc::new(FakeBackend::new()));
        AppState::new(backends, Arc::new(ManualClock::new(0)))
    }

    #[test]
    fn backend_resolves_a_registered_id() {
        let state = state_with_fake();
        assert!(state.backend("sandbox").is_ok());
    }

    #[test]
    fn backend_unknown_id_is_not_found_even_when_others_exist() {
        let state = state_with_fake();
        let err = state
            .backend("nope")
            .err()
            .expect("expected a notFound error");
        assert_eq!(err.kind, "notFound");
    }

    #[test]
    fn backend_on_empty_state_is_not_found() {
        let state = empty_state();
        let err = state
            .backend("sandbox")
            .err()
            .expect("expected a notFound error");
        assert_eq!(err.kind, "notFound");
    }

    #[tokio::test]
    async fn replace_task_aborts_the_previous_task_and_keeps_the_new_one() {
        let state = empty_state();

        let first = tokio::spawn(future::pending::<()>());
        state.replace_task("sandbox".to_owned(), first.abort_handle());

        let second = tokio::spawn(future::pending::<()>());
        state.replace_task("sandbox".to_owned(), second.abort_handle());

        let joined = first.await;
        assert!(
            joined.is_err() && joined.unwrap_err().is_cancelled(),
            "old task not aborted"
        );
        assert!(!second.is_finished(), "new task must stay alive");
        assert_eq!(
            state.tasks.lock().unwrap().len(),
            1,
            "exactly one live task"
        );

        second.abort();
    }

    #[tokio::test]
    async fn abort_task_cancels_and_removes_the_task() {
        let state = empty_state();
        let task = tokio::spawn(future::pending::<()>());
        state.replace_task("sandbox".to_owned(), task.abort_handle());

        state.abort_task("sandbox");

        let joined = task.await;
        assert!(joined.is_err() && joined.unwrap_err().is_cancelled());
        assert!(state.tasks.lock().unwrap().is_empty());
    }
}
