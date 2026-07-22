use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Age/duration in whole seconds.
pub type Seconds = u64;

/// Opaque backend-provided JSON payload (job data, output, extension values).
pub type Json = serde_json::Value;

/// Identifier of a job, serialized transparently as a bare string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub String);

impl From<&str> for JobId {
    fn from(value: &str) -> Self {
        JobId(value.to_owned())
    }
}

impl From<String> for JobId {
    fn from(value: String) -> Self {
        JobId(value)
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Job lifecycle states (PRD F4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Created,
    Active,
    Completed,
    Failed,
    Cancelled,
    Retry,
    DeadLetter,
}

impl JobState {
    /// The states that count as "waiting" for oldest-waiting-age purposes.
    pub fn is_waiting(self) -> bool {
        matches!(self, JobState::Created | JobState::Retry)
    }

    fn wire_str(self) -> &'static str {
        match self {
            JobState::Created => "created",
            JobState::Active => "active",
            JobState::Completed => "completed",
            JobState::Failed => "failed",
            JobState::Cancelled => "cancelled",
            JobState::Retry => "retry",
            JobState::DeadLetter => "deadLetter",
        }
    }
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.wire_str())
    }
}

/// Aggregate view of a single queue. [`QueueSummary::new`] derives `total_depth`
/// from the per-state counts; deserialized values are validated by C4's
/// conformance suite rather than at construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueSummary {
    pub name: String,
    pub total_depth: u64,
    pub counts_by_state: BTreeMap<JobState, u64>,
    pub oldest_waiting_age: Option<Seconds>,
}

impl QueueSummary {
    /// Build a summary, deriving `total_depth` as the saturating sum of the
    /// per-state counts (hostile/huge counts cannot overflow-panic).
    pub fn new(
        name: impl Into<String>,
        counts_by_state: BTreeMap<JobState, u64>,
        oldest_waiting_age: Option<Seconds>,
    ) -> Self {
        let total_depth = counts_by_state
            .values()
            .fold(0u64, |acc, count| acc.saturating_add(*count));
        Self {
            name: name.into(),
            total_depth,
            counts_by_state,
            oldest_waiting_age,
        }
    }
}

/// Age of the oldest still-waiting job, considering only `Created`/`Retry` jobs.
/// Returns `None` when nothing is waiting.
pub fn oldest_waiting_age<I>(jobs: I) -> Option<Seconds>
where
    I: IntoIterator<Item = (JobState, Seconds)>,
{
    jobs.into_iter()
        .filter(|(state, _)| state.is_waiting())
        .map(|(_, age)| age)
        .max()
}

/// Compact job row for list views. Timestamps are epoch milliseconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSummary {
    pub id: JobId,
    pub queue: String,
    pub state: JobState,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub attempts: u32,
    pub priority: i32,
}

/// A single lifecycle transition; `at` is epoch milliseconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    pub at: u64,
    pub state: JobState,
}

/// Retry accounting for a job; `next_retry_at` is epoch milliseconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryReadout {
    pub attempts: u32,
    pub max_attempts: Option<u32>,
    pub next_retry_at: Option<u64>,
}

/// Full job view. `extensions` carries backend-specific fields verbatim so the
/// core model stays honest across backends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDetail {
    #[serde(flatten)]
    pub summary: JobSummary,
    pub data: Json,
    pub output: Json,
    pub timeline: Vec<TimelineEvent>,
    pub retry: RetryReadout,
    pub extensions: BTreeMap<String, Json>,
}

/// Result of a connection probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendInfo {
    pub name: String,
    pub healthy: bool,
    pub detail: Option<String>,
}

/// Feature flags a backend supports; drives capability-aware rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub priority: bool,
    pub singleton: bool,
    pub dead_letter: bool,
    pub extensions: Vec<String>,
}

/// Inclusive epoch-millisecond window used to filter jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeWindow {
    pub from: u64,
    pub to: u64,
}

/// Filter + cursor for `list_jobs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobFilter {
    pub queue: Option<String>,
    pub states: Option<Vec<JobState>>,
    pub time_window: Option<TimeWindow>,
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn wire(state: JobState) -> String {
        serde_json::to_string(&state).unwrap()
    }

    #[test]
    fn job_state_serializes_to_camel_case_wire_strings() {
        assert_eq!(wire(JobState::Created), "\"created\"");
        assert_eq!(wire(JobState::Active), "\"active\"");
        assert_eq!(wire(JobState::Completed), "\"completed\"");
        assert_eq!(wire(JobState::Failed), "\"failed\"");
        assert_eq!(wire(JobState::Cancelled), "\"cancelled\"");
        assert_eq!(wire(JobState::Retry), "\"retry\"");
        assert_eq!(wire(JobState::DeadLetter), "\"deadLetter\"");
    }

    #[test]
    fn job_state_round_trips_through_serde() {
        for state in [
            JobState::Created,
            JobState::Active,
            JobState::Completed,
            JobState::Failed,
            JobState::Cancelled,
            JobState::Retry,
            JobState::DeadLetter,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: JobState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }

    #[test]
    fn job_state_display_matches_wire_strings() {
        assert_eq!(JobState::Created.to_string(), "created");
        assert_eq!(JobState::DeadLetter.to_string(), "deadLetter");
        assert_eq!(JobState::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn job_state_is_camel_case_as_map_key() {
        let counts = BTreeMap::from([(JobState::DeadLetter, 1u64)]);
        assert_eq!(
            serde_json::to_string(&counts).unwrap(),
            "{\"deadLetter\":1}"
        );
    }

    #[test]
    fn job_id_display_and_conversions_round_trip() {
        assert_eq!(JobId::from("abc").to_string(), "abc");
        assert_eq!(JobId::from("abc".to_owned()), JobId("abc".to_owned()));
    }

    #[test]
    fn is_waiting_is_true_only_for_created_and_retry() {
        assert!(JobState::Created.is_waiting());
        assert!(JobState::Retry.is_waiting());
        assert!(!JobState::Active.is_waiting());
        assert!(!JobState::Completed.is_waiting());
        assert!(!JobState::Failed.is_waiting());
        assert!(!JobState::Cancelled.is_waiting());
        assert!(!JobState::DeadLetter.is_waiting());
    }

    #[test]
    fn queue_summary_new_derives_total_depth_from_counts() {
        let counts = BTreeMap::from([
            (JobState::Created, 3u64),
            (JobState::Active, 2),
            (JobState::DeadLetter, 1),
        ]);
        let summary = QueueSummary::new("emails", counts.clone(), None);
        assert_eq!(summary.total_depth, 6);
        assert_eq!(summary.counts_by_state, counts);
        assert_eq!(summary.name, "emails");
    }

    #[test]
    fn queue_summary_total_depth_is_zero_for_empty_counts() {
        let summary = QueueSummary::new("empty", BTreeMap::new(), None);
        assert_eq!(summary.total_depth, 0);
    }

    #[test]
    fn oldest_waiting_age_is_max_age_among_created_and_retry() {
        let jobs = [
            (JobState::Created, 10u64),
            (JobState::Retry, 30),
            (JobState::Active, 100),
            (JobState::Completed, 500),
        ];
        assert_eq!(oldest_waiting_age(jobs), Some(30));
    }

    #[test]
    fn oldest_waiting_age_is_none_when_nothing_waiting() {
        let jobs = [(JobState::Active, 5u64), (JobState::Completed, 9)];
        assert_eq!(oldest_waiting_age(jobs), None);
    }

    #[test]
    fn job_detail_preserves_unknown_extension_keys() {
        let mut extensions = BTreeMap::new();
        extensions.insert(
            "singletonKey".to_owned(),
            serde_json::json!("welcome-email"),
        );
        extensions.insert("policy".to_owned(), serde_json::json!({ "retryLimit": 5 }));

        let detail = JobDetail {
            summary: JobSummary {
                id: JobId("job-1".to_owned()),
                queue: "emails".to_owned(),
                state: JobState::Active,
                created_at: 1,
                started_at: Some(2),
                completed_at: None,
                attempts: 1,
                priority: 0,
            },
            data: serde_json::json!({ "to": "a@b.com" }),
            output: serde_json::Value::Null,
            timeline: vec![
                TimelineEvent {
                    at: 1,
                    state: JobState::Created,
                },
                TimelineEvent {
                    at: 2,
                    state: JobState::Active,
                },
            ],
            retry: RetryReadout {
                attempts: 1,
                max_attempts: Some(3),
                next_retry_at: None,
            },
            extensions: extensions.clone(),
        };

        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("singletonKey"), "{json}");
        let back: JobDetail = serde_json::from_str(&json).unwrap();
        assert_eq!(back.extensions, extensions);
        assert_eq!(back, detail);
    }

    #[test]
    fn queue_summary_total_depth_saturates_on_overflow() {
        let counts = BTreeMap::from([(JobState::Created, u64::MAX), (JobState::Active, 10)]);
        let summary = QueueSummary::new("huge", counts, None);
        assert_eq!(summary.total_depth, u64::MAX);
    }

    fn assert_wire_keys(json: &str, keys: &[&str]) {
        for key in keys {
            assert!(json.contains(key), "missing {key} in {json}");
        }
    }

    #[test]
    fn queue_summary_serializes_expected_wire_keys() {
        let summary = QueueSummary::new(
            "emails",
            BTreeMap::from([(JobState::Created, 1u64)]),
            Some(9),
        );
        let json = serde_json::to_string(&summary).unwrap();
        assert_wire_keys(
            &json,
            &[
                "\"name\":",
                "\"totalDepth\":",
                "\"countsByState\":",
                "\"oldestWaitingAge\":",
            ],
        );
    }

    #[test]
    fn job_summary_serializes_expected_wire_keys() {
        let summary = JobSummary {
            id: JobId("j".to_owned()),
            queue: "q".to_owned(),
            state: JobState::Active,
            created_at: 1,
            started_at: Some(2),
            completed_at: Some(3),
            attempts: 4,
            priority: 5,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert_wire_keys(
            &json,
            &[
                "\"id\":",
                "\"queue\":",
                "\"state\":",
                "\"createdAt\":",
                "\"startedAt\":",
                "\"completedAt\":",
                "\"attempts\":",
                "\"priority\":",
            ],
        );
    }

    #[test]
    fn job_detail_serializes_expected_wire_keys() {
        let detail = JobDetail {
            summary: JobSummary {
                id: JobId("j".to_owned()),
                queue: "q".to_owned(),
                state: JobState::Active,
                created_at: 1,
                started_at: None,
                completed_at: None,
                attempts: 0,
                priority: 0,
            },
            data: serde_json::Value::Null,
            output: serde_json::Value::Null,
            timeline: Vec::new(),
            retry: RetryReadout {
                attempts: 0,
                max_attempts: None,
                next_retry_at: None,
            },
            extensions: BTreeMap::new(),
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert_wire_keys(
            &json,
            &[
                "\"createdAt\":",
                "\"data\":",
                "\"output\":",
                "\"timeline\":",
                "\"retry\":",
                "\"extensions\":",
            ],
        );
    }

    #[test]
    fn retry_readout_serializes_expected_wire_keys() {
        let retry = RetryReadout {
            attempts: 1,
            max_attempts: Some(3),
            next_retry_at: Some(9),
        };
        let json = serde_json::to_string(&retry).unwrap();
        assert_wire_keys(
            &json,
            &["\"attempts\":", "\"maxAttempts\":", "\"nextRetryAt\":"],
        );
    }

    #[test]
    fn capabilities_serializes_expected_wire_keys() {
        let capabilities = Capabilities {
            priority: true,
            singleton: false,
            dead_letter: true,
            extensions: Vec::new(),
        };
        let json = serde_json::to_string(&capabilities).unwrap();
        assert_wire_keys(
            &json,
            &[
                "\"priority\":",
                "\"singleton\":",
                "\"deadLetter\":",
                "\"extensions\":",
            ],
        );
    }

    #[test]
    fn job_filter_serializes_expected_wire_keys() {
        let filter = JobFilter {
            queue: Some("q".to_owned()),
            states: None,
            time_window: Some(TimeWindow { from: 1, to: 2 }),
            search: None,
            cursor: None,
            limit: 10,
        };
        let json = serde_json::to_string(&filter).unwrap();
        assert_wire_keys(
            &json,
            &[
                "\"queue\":",
                "\"states\":",
                "\"timeWindow\":",
                "\"search\":",
                "\"cursor\":",
                "\"limit\":",
            ],
        );
    }
}
