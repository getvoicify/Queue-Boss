//! Reusable fixtures for downstream crates (e.g. `src-tauri` in C5). This module
//! is deliberately NOT `#[cfg(test)]`-gated so it ships in the library.

use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::backend::QueueBackend;
use crate::error::BackendError;
use crate::model::{
    BackendInfo, Capabilities, JobDetail, JobFilter, JobId, JobState, JobSummary, QueueSummary,
    RetryReadout, TimelineEvent,
};
use crate::page::Page;

/// A canned [`QueueBackend`] returning deterministic fixtures.
#[derive(Debug, Default, Clone)]
pub struct FakeBackend;

impl FakeBackend {
    pub fn new() -> Self {
        Self
    }

    fn sample_summary(&self, id: JobId) -> JobSummary {
        JobSummary {
            id,
            queue: "default".to_owned(),
            state: JobState::Active,
            created_at: 1_700_000_000_000,
            started_at: Some(1_700_000_001_000),
            completed_at: None,
            attempts: 1,
            priority: 0,
        }
    }
}

#[async_trait]
impl QueueBackend for FakeBackend {
    async fn test_connection(&self) -> Result<BackendInfo, BackendError> {
        Ok(BackendInfo {
            name: "fake".to_owned(),
            healthy: true,
            detail: None,
        })
    }

    async fn list_queues(&self) -> Result<Vec<QueueSummary>, BackendError> {
        let counts = BTreeMap::from([(JobState::Created, 2u64), (JobState::Active, 1)]);
        Ok(vec![QueueSummary::new("default", counts, Some(42))])
    }

    async fn list_jobs(&self, _filter: JobFilter) -> Result<Page<JobSummary>, BackendError> {
        Ok(Page {
            items: vec![self.sample_summary(JobId("job-1".to_owned()))],
            next_cursor: None,
            has_more: false,
        })
    }

    async fn get_job(&self, id: &JobId) -> Result<JobDetail, BackendError> {
        Ok(JobDetail {
            summary: self.sample_summary(id.clone()),
            data: serde_json::Value::Null,
            output: serde_json::Value::Null,
            timeline: vec![TimelineEvent {
                at: 1_700_000_000_000,
                state: JobState::Created,
            }],
            retry: RetryReadout {
                attempts: 1,
                max_attempts: Some(3),
                next_retry_at: None,
            },
            extensions: BTreeMap::new(),
        })
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            priority: true,
            singleton: false,
            dead_letter: true,
            extensions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_backend_is_object_safe_and_usable() {
        let backend: Box<dyn QueueBackend> = Box::new(FakeBackend::new());

        let info = backend.test_connection().await.unwrap();
        assert_eq!(info.name, "fake");
        assert!(info.healthy);

        let queues = backend.list_queues().await.unwrap();
        assert_eq!(queues.len(), 1);
        assert_eq!(queues[0].total_depth, 3);
    }

    #[tokio::test]
    async fn fake_backend_get_job_echoes_id() {
        let backend = FakeBackend::new();
        let detail = backend.get_job(&JobId("abc".to_owned())).await.unwrap();
        assert_eq!(detail.summary.id, JobId("abc".to_owned()));
    }
}
