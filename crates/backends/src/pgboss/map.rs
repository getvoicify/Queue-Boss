//! Pure mapping between the pg-boss v10 wire shapes and the core model. Kept
//! free of I/O so the version band, the derived dead-letter bucketing, and the
//! summary assembly are unit-testable in-process (and mutation-tested) without a
//! database.

use std::collections::BTreeMap;

use qb_core::{BackendError, JobState, QueueSummary, Seconds};

use crate::pgboss::rows::StateCountRow;

/// Detected pg-boss schema flavor. Only `V10` is implemented in P0; E2-7 adds a
/// `V11` arm without touching the v10 path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaFlavor {
    V10,
}

impl SchemaFlavor {
    pub(crate) fn label(self) -> &'static str {
        match self {
            SchemaFlavor::V10 => "v10",
        }
    }
}

/// Classify a `pgboss.version.version` integer into a supported schema flavor.
/// `None` means the `version` table is absent (not a pg-boss schema at all). The
/// v10 supported band is schema versions 21–24 (floor 21); anything else —
/// including v11's 25 (until E2-7) and a missing table — is `Unsupported` with
/// self-authored product copy, never a driver string.
pub(crate) fn classify_version(version: Option<i32>) -> Result<SchemaFlavor, BackendError> {
    match version {
        Some(v) if (21..=24).contains(&v) => Ok(SchemaFlavor::V10),
        Some(v) => Err(BackendError::Unsupported(format!(
            "pg-boss v10 required (schema versions 21–24); found schema v{v}"
        ))),
        None => Err(BackendError::Unsupported(
            "pg-boss v10 required (schema versions 21–24); found no pgboss schema".to_owned(),
        )),
    }
}

/// Map a `qb_state` projection value (the six native pg-boss states plus the
/// derived `deadLetter`) to the core [`JobState`]. Unknown text -> `None`.
pub(crate) fn qb_state_from_str(state: &str) -> Option<JobState> {
    Some(match state {
        "created" => JobState::Created,
        "retry" => JobState::Retry,
        "active" => JobState::Active,
        "completed" => JobState::Completed,
        "cancelled" => JobState::Cancelled,
        "failed" => JobState::Failed,
        "deadLetter" => JobState::DeadLetter,
        _ => return None,
    })
}

/// Assemble per-queue [`QueueSummary`] values from the queue set, the grouped
/// state counts, and the precomputed oldest-waiting ages. Every queue in
/// `names` appears (drained queues get all-zero counts); each count row lands in
/// exactly one state bucket, so `QueueSummary::new`'s sum invariant holds.
pub(crate) fn build_summaries(
    names: &[String],
    counts: &[StateCountRow],
    ages: &BTreeMap<String, Option<Seconds>>,
) -> Result<Vec<QueueSummary>, BackendError> {
    let mut by_queue: BTreeMap<String, BTreeMap<JobState, u64>> = BTreeMap::new();
    for name in names {
        by_queue.entry(name.clone()).or_default();
    }
    for row in counts {
        let state = qb_state_from_str(&row.qb_state).ok_or_else(|| {
            BackendError::Internal("unexpected job state from pg-boss".to_owned())
        })?;
        let bucket = by_queue.entry(row.name.clone()).or_default();
        *bucket.entry(state).or_default() += row.size.max(0) as u64;
    }
    Ok(by_queue
        .into_iter()
        .map(|(name, counts_by_state)| {
            let age = ages.get(&name).copied().flatten();
            QueueSummary::new(name, counts_by_state, age)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_version_maps_the_v10_band_to_v10() {
        for v in [21, 22, 23, 24] {
            assert_eq!(
                classify_version(Some(v)).unwrap(),
                SchemaFlavor::V10,
                "v{v}"
            );
        }
    }

    #[test]
    fn classify_version_rejects_below_the_v10_floor() {
        let err = classify_version(Some(20)).unwrap_err();
        assert!(matches!(err, BackendError::Unsupported(_)));
        assert!(err.to_string().contains("v20"), "{err}");
    }

    #[test]
    fn classify_version_rejects_v11_until_e2_7() {
        let err = classify_version(Some(25)).unwrap_err();
        assert!(matches!(err, BackendError::Unsupported(_)));
        assert!(err.to_string().contains("v25"), "{err}");
    }

    #[test]
    fn classify_version_rejects_a_missing_version_table() {
        let err = classify_version(None).unwrap_err();
        assert!(matches!(err, BackendError::Unsupported(_)));
        assert!(err.to_string().contains("no pgboss schema"), "{err}");
    }

    #[test]
    fn qb_state_from_str_maps_all_native_and_derived_states() {
        assert_eq!(qb_state_from_str("created"), Some(JobState::Created));
        assert_eq!(qb_state_from_str("retry"), Some(JobState::Retry));
        assert_eq!(qb_state_from_str("active"), Some(JobState::Active));
        assert_eq!(qb_state_from_str("completed"), Some(JobState::Completed));
        assert_eq!(qb_state_from_str("cancelled"), Some(JobState::Cancelled));
        assert_eq!(qb_state_from_str("failed"), Some(JobState::Failed));
        assert_eq!(qb_state_from_str("deadLetter"), Some(JobState::DeadLetter));
        assert_eq!(qb_state_from_str("bogus"), None);
    }

    fn count(name: &str, qb_state: &str, size: i64) -> StateCountRow {
        StateCountRow {
            name: name.to_owned(),
            qb_state: qb_state.to_owned(),
            size,
        }
    }

    #[test]
    fn build_summaries_buckets_dead_letter_apart_from_failed_and_sums_depth() {
        let names = vec!["orders".to_owned(), "drained".to_owned()];
        let counts = vec![
            count("orders", "failed", 1),
            count("orders", "deadLetter", 2),
            count("orders", "created", 3),
        ];
        let ages = BTreeMap::from([
            ("orders".to_owned(), Some(9u64)),
            ("drained".to_owned(), None),
        ]);

        let summaries = build_summaries(&names, &counts, &ages).unwrap();

        let orders = summaries.iter().find(|s| s.name == "orders").unwrap();
        assert_eq!(orders.counts_by_state[&JobState::Failed], 1);
        assert_eq!(orders.counts_by_state[&JobState::DeadLetter], 2);
        assert_eq!(orders.counts_by_state[&JobState::Created], 3);
        assert_eq!(orders.total_depth, 6);
        assert_eq!(orders.oldest_waiting_age, Some(9));

        let drained = summaries.iter().find(|s| s.name == "drained").unwrap();
        assert!(drained.counts_by_state.is_empty());
        assert_eq!(drained.total_depth, 0);
        assert_eq!(drained.oldest_waiting_age, None);
    }

    #[test]
    fn build_summaries_includes_queues_with_no_jobs() {
        let names = vec!["empty".to_owned()];
        let summaries = build_summaries(&names, &[], &BTreeMap::new()).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, "empty");
        assert_eq!(summaries[0].total_depth, 0);
    }

    #[test]
    fn build_summaries_rejects_an_unknown_state() {
        let counts = vec![count("q", "weird", 1)];
        let err = build_summaries(&["q".to_owned()], &counts, &BTreeMap::new()).unwrap_err();
        assert!(matches!(err, BackendError::Internal(_)));
    }
}
