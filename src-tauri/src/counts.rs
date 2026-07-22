use std::collections::BTreeMap;

use qb_core::{JobState, QueueSummary, Seconds};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueCounts {
    pub connection_id: String,
    pub queues: Vec<QueueCountEntry>,
    pub polled_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueCountEntry {
    pub queue: String,
    pub total_depth: u64,
    pub counts_by_state: BTreeMap<JobState, u64>,
    pub oldest_waiting_age: Option<Seconds>,
}

impl QueueCounts {
    pub fn from_summaries(
        connection_id: impl Into<String>,
        summaries: Vec<QueueSummary>,
        polled_at: u64,
    ) -> Self {
        let queues = summaries.into_iter().map(QueueCountEntry::from).collect();
        Self {
            connection_id: connection_id.into(),
            queues,
            polled_at,
        }
    }
}

impl From<QueueSummary> for QueueCountEntry {
    fn from(summary: QueueSummary) -> Self {
        Self {
            queue: summary.name,
            total_depth: summary.total_depth,
            counts_by_state: summary.counts_by_state,
            oldest_waiting_age: summary.oldest_waiting_age,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary() -> QueueSummary {
        let counts = BTreeMap::from([
            (JobState::Created, 3u64),
            (JobState::Active, 2),
            (JobState::DeadLetter, 1),
        ]);
        QueueSummary::new("emails", counts, Some(9))
    }

    #[test]
    fn from_summaries_maps_queue_summary_fields() {
        let counts = QueueCounts::from_summaries("sandbox", vec![summary()], 5000);

        assert_eq!(counts.connection_id, "sandbox");
        assert_eq!(counts.polled_at, 5000);
        assert_eq!(counts.queues.len(), 1);

        let entry = &counts.queues[0];
        assert_eq!(entry.queue, "emails");
        assert_eq!(entry.total_depth, 6);
        assert_eq!(entry.oldest_waiting_age, Some(9));
        assert_eq!(entry.counts_by_state.get(&JobState::DeadLetter), Some(&1));
    }

    #[test]
    fn serializes_pinned_camel_case_wire_keys() {
        let counts = QueueCounts::from_summaries("sandbox", vec![summary()], 5000);
        let json = serde_json::to_string(&counts).unwrap();

        for key in [
            "\"connectionId\":",
            "\"queues\":",
            "\"queue\":",
            "\"totalDepth\":",
            "\"countsByState\":",
            "\"deadLetter\":",
            "\"oldestWaitingAge\":",
            "\"polledAt\":",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
    }
}
