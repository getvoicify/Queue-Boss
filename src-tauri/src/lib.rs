mod commands;
mod counts;
mod poller;
mod state;

use std::collections::HashMap;
use std::sync::Arc;

use qb_backends::SandboxBackend;
use qb_core::{
    BackendInfo, Clock, JobDetail, JobFilter, JobSummary, Page, QueueBackend, QueueSummary,
    SystemClock,
};
use tauri::ipc::Channel;
use tauri::State;

use crate::commands::{
    get_job_impl, list_jobs_impl, list_queues_impl, subscribe_counts_impl, test_connection_impl,
    CommandError,
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

/// Assemble the Tauri-managed application state with the default backends.
fn build_app_state() -> AppState {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let mut backends: HashMap<ConnectionId, Arc<dyn QueueBackend>> = HashMap::new();
    backends.insert(
        "sandbox".to_owned(),
        Arc::new(SandboxBackend::new(clock.clone(), SANDBOX_SEED)) as Arc<dyn QueueBackend>,
    );
    AppState::new(backends, clock)
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
            subscribe_counts
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
}
