use async_trait::async_trait;

use crate::error::BackendError;
use crate::model::{
    BackendInfo, Capabilities, JobDetail, JobFilter, JobId, JobSummary, QueueSummary,
};
use crate::page::Page;

/// Read-only v1 contract every queue backend implements. Write/job-action
/// methods are intentionally absent and arrive with the post-MVP write path.
#[async_trait]
pub trait QueueBackend: Send + Sync {
    async fn test_connection(&self) -> Result<BackendInfo, BackendError>;
    async fn list_queues(&self) -> Result<Vec<QueueSummary>, BackendError>;
    async fn list_jobs(&self, filter: JobFilter) -> Result<Page<JobSummary>, BackendError>;
    async fn get_job(&self, id: &JobId) -> Result<JobDetail, BackendError>;
    fn capabilities(&self) -> Capabilities;
}
