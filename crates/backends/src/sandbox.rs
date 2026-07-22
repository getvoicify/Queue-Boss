use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use qb_core::{
    decode_cursor, encode_cursor, oldest_waiting_age, BackendError, BackendInfo, Capabilities,
    Clock, Cursor, JobDetail, JobFilter, JobId, JobState, JobSummary, Json, Page, QueueBackend,
    QueueSummary, RetryReadout,
};

use crate::simulator::{Job, Simulator};

/// An in-memory synthetic queue backend. Needs no database and no setup: it
/// derives a live producer/consumer picture from an injected [`Clock`] and a
/// seed. Deterministic under a `ManualClock` (tests), free-running under a
/// `SystemClock` (the app).
pub struct SandboxBackend {
    clock: Arc<dyn Clock>,
    sim: Simulator,
}

impl SandboxBackend {
    pub fn new(clock: Arc<dyn Clock>, seed: u64) -> Self {
        let epoch = clock.now_ms();
        Self {
            sim: Simulator::new(epoch, seed),
            clock,
        }
    }

    fn now(&self) -> u64 {
        self.clock.now_ms()
    }

    fn find_job(&self, now: u64, id: &JobId) -> Option<Job> {
        self.sim.jobs_at(now).into_iter().find(|job| job.id() == id)
    }
}

fn matches_filter(filter: &JobFilter, summary: &JobSummary) -> bool {
    if let Some(queue) = &filter.queue {
        if &summary.queue != queue {
            return false;
        }
    }
    if let Some(states) = &filter.states {
        if !states.contains(&summary.state) {
            return false;
        }
    }
    if let Some(window) = &filter.time_window {
        if summary.created_at < window.from || summary.created_at > window.to {
            return false;
        }
    }
    if let Some(search) = &filter.search {
        if !summary.id.0.contains(search) {
            return false;
        }
    }
    true
}

fn seconds_between(now: u64, then: u64) -> u64 {
    now.saturating_sub(then) / 1_000
}

#[async_trait]
impl QueueBackend for SandboxBackend {
    async fn test_connection(&self) -> Result<BackendInfo, BackendError> {
        Ok(BackendInfo {
            name: "sandbox".to_owned(),
            healthy: true,
            detail: Some("in-memory synthetic queue".to_owned()),
        })
    }

    async fn list_queues(&self) -> Result<Vec<QueueSummary>, BackendError> {
        let now = self.now();
        let mut counts: BTreeMap<String, BTreeMap<JobState, u64>> = BTreeMap::new();
        let mut waiting: BTreeMap<String, Vec<(JobState, u64)>> = BTreeMap::new();

        for job in self.sim.jobs_at(now) {
            let Some(summary) = job.summary_at(now) else {
                continue;
            };
            *counts
                .entry(summary.queue.clone())
                .or_default()
                .entry(summary.state)
                .or_default() += 1;
            waiting
                .entry(summary.queue.clone())
                .or_default()
                .push((summary.state, seconds_between(now, summary.created_at)));
        }

        let summaries = counts
            .into_iter()
            .map(|(name, counts_by_state)| {
                let oldest = oldest_waiting_age(waiting.get(&name).cloned().unwrap_or_default());
                QueueSummary::new(name, counts_by_state, oldest)
            })
            .collect();
        Ok(summaries)
    }

    async fn list_jobs(&self, filter: JobFilter) -> Result<Page<JobSummary>, BackendError> {
        let now = self.now();
        let mut jobs: Vec<JobSummary> = self
            .sim
            .jobs_at(now)
            .into_iter()
            .filter_map(|job| job.summary_at(now))
            .filter(|summary| matches_filter(&filter, summary))
            .collect();
        jobs.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));

        if let Some(encoded) = &filter.cursor {
            let cursor = decode_cursor(encoded)?;
            jobs.retain(|summary| {
                (summary.created_at, &summary.id) > (cursor.created_at, &cursor.id)
            });
        }

        let limit = filter.limit as usize;
        let has_more = limit > 0 && jobs.len() > limit;
        let items: Vec<JobSummary> = jobs.into_iter().take(limit).collect();
        let next_cursor = if has_more {
            items.last().map(|summary| {
                encode_cursor(&Cursor {
                    created_at: summary.created_at,
                    id: summary.id.clone(),
                })
            })
        } else {
            None
        };

        Ok(Page {
            items,
            next_cursor,
            has_more,
        })
    }

    async fn get_job(&self, id: &JobId) -> Result<JobDetail, BackendError> {
        let now = self.now();
        let job = self
            .find_job(now, id)
            .ok_or_else(|| BackendError::NotFound(format!("job {id} not found")))?;
        let summary = job
            .summary_at(now)
            .ok_or_else(|| BackendError::NotFound(format!("job {id} not found")))?;

        let state = summary.state;
        let retry = RetryReadout {
            attempts: summary.attempts,
            max_attempts: Some(job.max_attempts()),
            next_retry_at: if state == JobState::Retry {
                job.next_active_after(now)
            } else {
                None
            },
        };
        let output = if state == JobState::Completed {
            serde_json::json!({ "ok": true })
        } else {
            Json::Null
        };

        Ok(JobDetail {
            summary,
            data: serde_json::json!({ "index": job.index(), "queue": job.queue() }),
            output,
            timeline: job.timeline_at(now),
            retry,
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
    use qb_core::{ManualClock, TimeWindow};

    fn backend(start: u64, seed: u64) -> (ManualClock, SandboxBackend) {
        let clock = ManualClock::new(start);
        let backend = SandboxBackend::new(Arc::new(clock.clone()) as Arc<dyn Clock>, seed);
        (clock, backend)
    }

    #[tokio::test]
    async fn test_connection_reports_healthy_sandbox() {
        let (_clock, backend) = backend(0, 1);
        let info = backend.test_connection().await.unwrap();
        assert_eq!(info.name, "sandbox");
        assert!(info.healthy);
    }

    #[tokio::test]
    async fn capabilities_advertise_priority_and_dead_letter() {
        let (_clock, backend) = backend(0, 1);
        let caps = backend.capabilities();
        assert!(caps.priority);
        assert!(caps.dead_letter);
    }

    #[tokio::test]
    async fn get_job_for_unknown_id_is_not_found() {
        let (_clock, backend) = backend(0, 1);
        let err = backend.get_job(&JobId("nope-999".to_owned())).await;
        assert!(matches!(err, Err(BackendError::NotFound(_))));
    }

    #[tokio::test]
    async fn queue_depth_equals_visible_job_count() {
        let (clock, backend) = backend(1_000, 5);
        clock.advance(4_000);
        let queues = backend.list_queues().await.unwrap();
        let depth: u64 = queues.iter().map(|q| q.total_depth).sum();
        let jobs = backend
            .list_jobs(JobFilter {
                queue: None,
                states: None,
                time_window: None,
                search: None,
                cursor: None,
                limit: 1_000_000,
            })
            .await
            .unwrap();
        assert_eq!(depth, jobs.items.len() as u64);
    }

    #[tokio::test]
    async fn cursor_pages_do_not_overlap() {
        let (clock, backend) = backend(0, 3);
        clock.advance(6_000);
        let mk = |cursor: Option<String>| JobFilter {
            queue: None,
            states: None,
            time_window: None,
            search: None,
            cursor,
            limit: 5,
        };
        let first = backend.list_jobs(mk(None)).await.unwrap();
        assert!(first.has_more);
        let cursor = first.next_cursor.clone().unwrap();
        let second = backend.list_jobs(mk(Some(cursor))).await.unwrap();
        let first_ids: std::collections::BTreeSet<_> =
            first.items.iter().map(|j| j.id.clone()).collect();
        assert!(
            second.items.iter().all(|j| !first_ids.contains(&j.id)),
            "second page must not repeat first-page ids"
        );
    }

    #[tokio::test]
    async fn get_job_in_retry_reports_next_retry_at() {
        let (clock, backend) = backend(0, 11);
        let retry_filter = || JobFilter {
            queue: None,
            states: Some(vec![JobState::Retry]),
            time_window: None,
            search: None,
            cursor: None,
            limit: 1_000,
        };
        // Sweep simulated time until at least one job is backing off in Retry.
        let mut sample = None;
        for _ in 0..200 {
            let retrying = backend.list_jobs(retry_filter()).await.unwrap();
            if let Some(job) = retrying.items.first() {
                sample = Some(job.id.clone());
                break;
            }
            clock.advance(50);
        }
        let id = sample.expect("expected some job to enter Retry during the sweep");
        let detail = backend.get_job(&id).await.unwrap();
        assert_eq!(detail.summary.state, JobState::Retry);
        assert!(
            detail.retry.next_retry_at.is_some(),
            "a retrying job must schedule its next attempt"
        );
    }

    fn everything(cursor: Option<String>, limit: u32) -> JobFilter {
        JobFilter {
            queue: None,
            states: None,
            time_window: None,
            search: None,
            cursor,
            limit,
        }
    }

    async fn fetch_all(backend: &SandboxBackend) -> Vec<JobSummary> {
        backend
            .list_jobs(everything(None, 1_000_000))
            .await
            .unwrap()
            .items
    }

    #[test]
    fn seconds_between_converts_millis_to_whole_seconds() {
        assert_eq!(seconds_between(5_000, 1_000), 4);
        assert_eq!(seconds_between(1_900, 1_000), 0);
        assert_eq!(seconds_between(1_000, 5_000), 0, "saturates, no underflow");
    }

    #[tokio::test]
    async fn filters_by_queue() {
        let (clock, backend) = backend(1_000, 2);
        clock.advance(5_000);
        let all = fetch_all(&backend).await;
        let queue = all[0].queue.clone();
        let filtered = backend
            .list_jobs(JobFilter {
                queue: Some(queue.clone()),
                ..everything(None, 1_000_000)
            })
            .await
            .unwrap();
        assert!(!filtered.items.is_empty());
        assert!(filtered.items.iter().all(|j| j.queue == queue));
        let expected: std::collections::BTreeSet<_> = all
            .iter()
            .filter(|j| j.queue == queue)
            .map(|j| j.id.clone())
            .collect();
        let got: std::collections::BTreeSet<_> =
            filtered.items.iter().map(|j| j.id.clone()).collect();
        assert_eq!(got, expected);
    }

    #[tokio::test]
    async fn filters_by_time_window() {
        let (clock, backend) = backend(1_000, 2);
        clock.advance(6_000);
        let all = fetch_all(&backend).await;
        let window = TimeWindow {
            from: 1_000,
            to: 2_000,
        };
        let filtered = backend
            .list_jobs(JobFilter {
                time_window: Some(window.clone()),
                ..everything(None, 1_000_000)
            })
            .await
            .unwrap();
        let expected: std::collections::BTreeSet<_> = all
            .iter()
            .filter(|j| j.created_at >= window.from && j.created_at <= window.to)
            .map(|j| j.id.clone())
            .collect();
        let got: std::collections::BTreeSet<_> =
            filtered.items.iter().map(|j| j.id.clone()).collect();
        assert!(!got.is_empty(), "window must match some jobs");
        assert_eq!(got, expected, "window must match exactly the in-range jobs");
    }

    #[tokio::test]
    async fn filters_by_search_substring() {
        let (clock, backend) = backend(0, 2);
        clock.advance(5_000);
        let all = fetch_all(&backend).await;
        let filtered = backend
            .list_jobs(JobFilter {
                search: Some("batch".to_owned()),
                ..everything(None, 1_000_000)
            })
            .await
            .unwrap();
        assert!(!filtered.items.is_empty());
        assert!(filtered.items.iter().all(|j| j.id.0.contains("batch")));
        let expected: std::collections::BTreeSet<_> = all
            .iter()
            .filter(|j| j.id.0.contains("batch"))
            .map(|j| j.id.clone())
            .collect();
        let got: std::collections::BTreeSet<_> =
            filtered.items.iter().map(|j| j.id.clone()).collect();
        assert_eq!(got, expected);
    }

    #[tokio::test]
    async fn has_more_is_false_when_the_page_exactly_fits() {
        let (clock, backend) = backend(0, 2);
        clock.advance(5_000);
        let total = fetch_all(&backend).await.len() as u32;
        let page = backend.list_jobs(everything(None, total)).await.unwrap();
        assert_eq!(page.items.len() as u32, total);
        assert!(!page.has_more, "an exact-fit page must not report more");
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn zero_limit_yields_an_empty_finished_page() {
        let (clock, backend) = backend(0, 2);
        clock.advance(2_000);
        let page = backend.list_jobs(everything(None, 0)).await.unwrap();
        assert!(page.items.is_empty());
        assert!(!page.has_more);
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn get_job_reports_output_data_and_max_attempts() {
        let (clock, backend) = backend(0, 2);
        clock.advance(6_000);

        let completed = backend
            .list_jobs(JobFilter {
                states: Some(vec![JobState::Completed]),
                ..everything(None, 1_000_000)
            })
            .await
            .unwrap();
        let done = completed.items.first().expect("expected a completed job");
        let detail = backend.get_job(&done.id).await.unwrap();
        assert_eq!(detail.output, serde_json::json!({ "ok": true }));
        assert_eq!(detail.retry.max_attempts, Some(4));
        assert_eq!(detail.data["queue"], serde_json::json!(done.queue));

        let created = backend
            .list_jobs(JobFilter {
                states: Some(vec![JobState::Created]),
                ..everything(None, 1_000_000)
            })
            .await
            .unwrap();
        let waiting = created.items.first().expect("expected a created job");
        let detail = backend.get_job(&waiting.id).await.unwrap();
        assert_eq!(
            detail.output,
            Json::Null,
            "non-completed jobs have no output"
        );
    }
}
