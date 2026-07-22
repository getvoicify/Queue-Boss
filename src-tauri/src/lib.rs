mod commands;
mod counts;
mod poller;
mod state;

use std::collections::HashMap;
use std::sync::Arc;

use qb_core::{BackendInfo, JobDetail, JobFilter, JobSummary, Page, QueueSummary, SystemClock};
use tauri::ipc::Channel;
use tauri::State;

use crate::commands::{
    get_job_impl, list_jobs_impl, list_queues_impl, subscribe_counts_impl, test_connection_impl,
    CommandError,
};
use crate::counts::QueueCounts;
use crate::poller::{poll_loop, DEFAULT_POLL_INTERVAL_MS};
use crate::state::AppState;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // E1 boots with no connections; C8 registers the default "sandbox" backend.
    let app_state = AppState::new(HashMap::new(), Arc::new(SystemClock));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
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
