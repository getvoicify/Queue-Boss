//! Pure mapping between the pg-boss v10 wire shapes and the core model. Kept
//! free of I/O so the version band, the derived dead-letter bucketing, and the
//! summary assembly are unit-testable in-process (and mutation-tested) without a
//! database.

use std::collections::BTreeMap;

use qb_core::{
    BackendError, JobDetail, JobId, JobState, JobSummary, Json, QueueSummary, RetryReadout,
    Seconds, TimelineEvent,
};

use crate::pgboss::rows::{JobDetailRow, JobSummaryRow, StateCountRow};

/// Detected pg-boss schema flavor. `V10` and `V11` share the flavor-agnostic
/// read path; only the version band and the reported label differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaFlavor {
    V10,
    V11,
}

impl SchemaFlavor {
    pub(crate) fn label(self) -> &'static str {
        match self {
            SchemaFlavor::V10 => "v10",
            SchemaFlavor::V11 => "v11",
        }
    }
}

/// Classify a `pgboss.version.version` integer into a supported schema flavor.
/// `None` means the `version` table is absent (not a pg-boss schema at all). The
/// supported band is schema versions 21–25: 21–24 → v10, 25 → v11. Anything else
/// — below the floor, above the ceiling, or a missing table — is `Unsupported`
/// with self-authored product copy, never a driver string.
pub(crate) fn classify_version(version: Option<i32>) -> Result<SchemaFlavor, BackendError> {
    match version {
        Some(v) if (21..=24).contains(&v) => Ok(SchemaFlavor::V10),
        Some(25) => Ok(SchemaFlavor::V11),
        Some(v) => Err(BackendError::Unsupported(format!(
            "pg-boss v10 or v11 required (schema versions 21–25); found schema v{v}"
        ))),
        None => Err(BackendError::Unsupported(
            "pg-boss v10 or v11 required (schema versions 21–25); found no pgboss schema"
                .to_owned(),
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

/// Clamp a possibly-negative epoch-ms `bigint` into the non-negative u64 the
/// core model uses. The seed timestamps are adversarial, so a raw negative value
/// must never wrap.
fn i64_to_ms(v: i64) -> u64 {
    v.max(0) as u64
}

/// Map a `list_jobs` row to a [`JobSummary`]. `attempts` is the pg-boss
/// `retry_count`; `priority` is preserved verbatim (the core field is signed).
pub(crate) fn to_summary(row: JobSummaryRow) -> Result<JobSummary, BackendError> {
    let state = qb_state_from_str(&row.qb_state)
        .ok_or_else(|| BackendError::Internal("unexpected job state from pg-boss".to_owned()))?;
    Ok(JobSummary {
        id: JobId(row.job_id),
        queue: row.name,
        state,
        created_at: i64_to_ms(row.created_at),
        started_at: row.started_at.map(i64_to_ms),
        completed_at: row.completed_at.map(i64_to_ms),
        attempts: row.attempts.max(0) as u32,
        priority: row.priority,
    })
}

/// Synthesize an ordered, valid-transition timeline for the current state.
/// `Active -> Retry` and `Active -> DeadLetter` are not whitelisted, so those
/// chains route through a synthesized `Failed` step. The source timestamps are
/// adversarial (seed sets `started_on < created_on`), so each `at` is clamped
/// to be non-decreasing rather than trusted.
fn synthesize_timeline(
    state: JobState,
    created_at: u64,
    started_at: Option<u64>,
    completed_at: Option<u64>,
) -> Vec<TimelineEvent> {
    use JobState::*;
    let skeleton: Vec<(JobState, Option<u64>)> = match state {
        Created => vec![(Created, Some(created_at))],
        Active => vec![(Created, Some(created_at)), (Active, started_at)],
        Completed => vec![
            (Created, Some(created_at)),
            (Active, started_at),
            (Completed, completed_at),
        ],
        Cancelled if started_at.is_some() => vec![
            (Created, Some(created_at)),
            (Active, started_at),
            (Cancelled, completed_at),
        ],
        Cancelled => vec![(Created, Some(created_at)), (Cancelled, completed_at)],
        Failed => vec![
            (Created, Some(created_at)),
            (Active, started_at),
            (Failed, completed_at),
        ],
        Retry => vec![
            (Created, Some(created_at)),
            (Active, started_at),
            (Failed, completed_at),
            (Retry, None),
        ],
        DeadLetter => vec![
            (Created, Some(created_at)),
            (Active, started_at),
            (Failed, completed_at),
            (DeadLetter, completed_at),
        ],
    };
    let mut out = Vec::with_capacity(skeleton.len());
    let mut prev = created_at;
    for (s, anchor) in skeleton {
        let at = anchor.unwrap_or(prev).max(prev);
        out.push(TimelineEvent { at, state: s });
        prev = at;
    }
    out
}

/// A nullable text column as a JSON string (or `Json::Null`), used for the
/// singleton/policy extension fields.
fn opt_str_json(v: Option<String>) -> Json {
    v.map(Json::from).unwrap_or(Json::Null)
}

/// Parse a `jsonb::text` payload into `Json`, defaulting to `Json::Null` when
/// absent or unparseable.
fn parse_json(v: Option<String>) -> Json {
    v.and_then(|t| serde_json::from_str::<Json>(&t).ok())
        .unwrap_or(Json::Null)
}

/// Map a `get_job` row to a full [`JobDetail`]: the flattened summary, parsed
/// `data`/`output`, the synthesized timeline, retry accounting (`next_retry_at`
/// only for the `Retry` state), and the backend-specific extension map (with a
/// `deadLetter` entry only when the job carries a dead-letter route).
pub(crate) fn to_detail(row: JobDetailRow) -> Result<JobDetail, BackendError> {
    let state = qb_state_from_str(&row.qb_state)
        .ok_or_else(|| BackendError::Internal("unexpected job state from pg-boss".to_owned()))?;
    let created_at = i64_to_ms(row.created_at);
    let started_at = row.started_at.map(i64_to_ms);
    let completed_at = row.completed_at.map(i64_to_ms);
    let summary = JobSummary {
        id: JobId(row.job_id),
        queue: row.name,
        state,
        created_at,
        started_at,
        completed_at,
        attempts: row.attempts.max(0) as u32,
        priority: row.priority,
    };
    let timeline = synthesize_timeline(state, created_at, started_at, completed_at);
    let retry = RetryReadout {
        attempts: row.attempts.max(0) as u32,
        max_attempts: Some(row.retry_limit.max(0) as u32),
        next_retry_at: if state == JobState::Retry {
            Some(i64_to_ms(row.start_after_ms))
        } else {
            None
        },
    };
    let extensions: BTreeMap<String, Json> = [
        ("singletonKey".to_owned(), opt_str_json(row.singleton_key)),
        ("policy".to_owned(), opt_str_json(row.policy)),
        ("priority".to_owned(), Json::from(row.priority)),
    ]
    .into_iter()
    .chain(
        row.dead_letter
            .map(|dl| ("deadLetter".to_owned(), Json::from(dl))),
    )
    .collect();
    Ok(JobDetail {
        summary,
        data: parse_json(row.data),
        output: parse_json(row.output),
        timeline,
        retry,
        extensions,
    })
}

/// Split rows fetched with the fetch-one-past strategy (`LIMIT = limit + 1`) into
/// the page items and whether a further page exists. `limit == 0` yields an empty
/// page that never reports more, preserving `has_more == next_cursor.is_some()`.
pub(crate) fn paginate<T>(fetched: Vec<T>, limit: u32) -> (Vec<T>, bool) {
    let has_more = limit > 0 && fetched.len() as u64 > limit as u64;
    let items: Vec<T> = fetched.into_iter().take(limit as usize).collect();
    (items, has_more)
}

/// Whether `s` is a syntactically valid UUID (a malformed job id is treated as absent).
pub(crate) fn is_uuid(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok()
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
    fn classify_version_maps_25_to_v11() {
        assert_eq!(classify_version(Some(25)).unwrap(), SchemaFlavor::V11);
    }

    #[test]
    fn classify_version_rejects_above_the_v11_ceiling() {
        let err = classify_version(Some(26)).unwrap_err();
        assert!(matches!(err, BackendError::Unsupported(_)));
        assert!(err.to_string().contains("v26"), "{err}");
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

    fn summary_row(qb_state: &str) -> JobSummaryRow {
        JobSummaryRow {
            created_at: 1_700_000_000_123,
            job_id: "job-7".to_owned(),
            name: "orders".to_owned(),
            qb_state: qb_state.to_owned(),
            started_at: Some(1_700_000_050_000),
            completed_at: None,
            attempts: 3,
            priority: 5,
        }
    }

    #[test]
    fn to_summary_maps_derived_dead_letter_state_and_epoch_ms_fields() {
        let summary = to_summary(summary_row("deadLetter")).unwrap();
        assert_eq!(summary.state, JobState::DeadLetter);
        assert_eq!(summary.id, qb_core::JobId("job-7".to_owned()));
        assert_eq!(summary.queue, "orders");
        assert_eq!(summary.created_at, 1_700_000_000_123);
        assert_eq!(summary.started_at, Some(1_700_000_050_000));
        assert_eq!(summary.completed_at, None);
        assert_eq!(summary.attempts, 3);
        assert_eq!(summary.priority, 5);
    }

    #[test]
    fn to_summary_maps_a_native_state_and_clamps_negative_timestamps() {
        let row = JobSummaryRow {
            created_at: -1,
            started_at: Some(-5),
            ..summary_row("created")
        };
        let summary = to_summary(row).unwrap();
        assert_eq!(summary.state, JobState::Created);
        assert_eq!(summary.created_at, 0);
        assert_eq!(summary.started_at, Some(0));
    }

    #[test]
    fn to_summary_rejects_an_unknown_state() {
        let err = to_summary(summary_row("weird")).unwrap_err();
        assert!(matches!(err, BackendError::Internal(_)));
    }

    fn detail_row(qb_state: &str) -> JobDetailRow {
        JobDetailRow {
            created_at: 1_700_000_000_000,
            job_id: "job-9".to_owned(),
            name: "orders".to_owned(),
            qb_state: qb_state.to_owned(),
            started_at: Some(1_700_000_050_000),
            completed_at: Some(1_700_000_060_000),
            attempts: 2,
            priority: 5,
            retry_limit: 4,
            start_after_ms: 1_700_000_070_000,
            singleton_key: None,
            policy: Some("standard".to_owned()),
            dead_letter: None,
            data: Some("{\"ok\":true}".to_owned()),
            output: None,
        }
    }

    // Local mirror of the conformance whitelist so the timeline tests prove the
    // synthesized chain is walkable, independent of the harness.
    fn is_valid_transition(from: JobState, to: JobState) -> bool {
        use JobState::*;
        matches!(
            (from, to),
            (Created, Active)
                | (Created, Cancelled)
                | (Active, Completed)
                | (Active, Failed)
                | (Active, Cancelled)
                | (Failed, Retry)
                | (Failed, DeadLetter)
                | (Retry, Active)
                | (Retry, Cancelled)
        )
    }

    fn assert_ordered_valid_chain(events: &[TimelineEvent], current: JobState) {
        assert!(!events.is_empty(), "timeline must be non-empty");
        assert_eq!(events[0].state, JobState::Created, "first must be Created");
        assert_eq!(
            events.last().unwrap().state,
            current,
            "last must be the current state"
        );
        for pair in events.windows(2) {
            assert!(
                pair[0].at <= pair[1].at,
                "at must be non-decreasing: {} then {}",
                pair[0].at,
                pair[1].at
            );
            assert!(
                is_valid_transition(pair[0].state, pair[1].state),
                "illegal transition {:?} -> {:?}",
                pair[0].state,
                pair[1].state
            );
        }
    }

    fn states_of(events: &[TimelineEvent]) -> Vec<JobState> {
        events.iter().map(|e| e.state).collect()
    }

    // Adversarial anchors: started_at (990) precedes created_at (1000), and
    // completed_at (2000) is the only forward point. The monotonic clamp must
    // still yield a non-decreasing `at` sequence for every current state.
    const CREATED_AT: u64 = 1000;
    const STARTED_AT: Option<u64> = Some(990);
    const COMPLETED_AT: Option<u64> = Some(2000);

    #[test]
    fn timeline_for_created_is_created_only() {
        let tl = synthesize_timeline(JobState::Created, CREATED_AT, STARTED_AT, COMPLETED_AT);
        assert_eq!(states_of(&tl), vec![JobState::Created]);
        assert_ordered_valid_chain(&tl, JobState::Created);
    }

    #[test]
    fn timeline_for_active_is_created_then_active() {
        let tl = synthesize_timeline(JobState::Active, CREATED_AT, STARTED_AT, COMPLETED_AT);
        assert_eq!(states_of(&tl), vec![JobState::Created, JobState::Active]);
        assert_ordered_valid_chain(&tl, JobState::Active);
    }

    #[test]
    fn timeline_for_completed_runs_created_active_completed() {
        let tl = synthesize_timeline(JobState::Completed, CREATED_AT, STARTED_AT, COMPLETED_AT);
        assert_eq!(
            states_of(&tl),
            vec![JobState::Created, JobState::Active, JobState::Completed]
        );
        assert_ordered_valid_chain(&tl, JobState::Completed);
    }

    #[test]
    fn timeline_for_cancelled_after_active_includes_active() {
        let tl = synthesize_timeline(JobState::Cancelled, CREATED_AT, STARTED_AT, COMPLETED_AT);
        assert_eq!(
            states_of(&tl),
            vec![JobState::Created, JobState::Active, JobState::Cancelled]
        );
        assert_ordered_valid_chain(&tl, JobState::Cancelled);
    }

    #[test]
    fn timeline_for_cancelled_without_start_omits_active() {
        let tl = synthesize_timeline(JobState::Cancelled, CREATED_AT, None, COMPLETED_AT);
        assert_eq!(states_of(&tl), vec![JobState::Created, JobState::Cancelled]);
        assert_ordered_valid_chain(&tl, JobState::Cancelled);
    }

    #[test]
    fn timeline_for_failed_runs_created_active_failed() {
        let tl = synthesize_timeline(JobState::Failed, CREATED_AT, STARTED_AT, COMPLETED_AT);
        assert_eq!(
            states_of(&tl),
            vec![JobState::Created, JobState::Active, JobState::Failed]
        );
        assert_ordered_valid_chain(&tl, JobState::Failed);
    }

    #[test]
    fn timeline_for_retry_routes_through_failed() {
        let tl = synthesize_timeline(JobState::Retry, CREATED_AT, STARTED_AT, COMPLETED_AT);
        assert_eq!(
            states_of(&tl),
            vec![
                JobState::Created,
                JobState::Active,
                JobState::Failed,
                JobState::Retry
            ]
        );
        assert_ordered_valid_chain(&tl, JobState::Retry);
    }

    #[test]
    fn timeline_for_dead_letter_routes_through_failed() {
        let tl = synthesize_timeline(JobState::DeadLetter, CREATED_AT, STARTED_AT, COMPLETED_AT);
        assert_eq!(
            states_of(&tl),
            vec![
                JobState::Created,
                JobState::Active,
                JobState::Failed,
                JobState::DeadLetter
            ]
        );
        assert_ordered_valid_chain(&tl, JobState::DeadLetter);
    }

    #[test]
    fn mirror_accepts_every_authoritative_transition() {
        use JobState::*;
        let authoritative = [
            (Created, Active),
            (Created, Cancelled),
            (Active, Completed),
            (Active, Failed),
            (Active, Cancelled),
            (Failed, Retry),
            (Failed, DeadLetter),
            (Retry, Active),
            (Retry, Cancelled),
        ];
        for (from, to) in authoritative {
            assert!(
                is_valid_transition(from, to),
                "{from:?} -> {to:?} must be accepted"
            );
        }
    }

    #[test]
    fn to_detail_retry_row_exposes_next_retry_and_max_attempts() {
        let detail = to_detail(detail_row("retry")).unwrap();
        assert_eq!(detail.retry.next_retry_at, Some(1_700_000_070_000));
        assert_eq!(detail.retry.max_attempts, Some(4));
        assert_eq!(detail.retry.attempts, 2);
    }

    #[test]
    fn to_detail_non_retry_row_has_no_next_retry() {
        let detail = to_detail(detail_row("active")).unwrap();
        assert_eq!(detail.retry.next_retry_at, None);
    }

    #[test]
    fn to_detail_routed_row_records_the_dead_letter_extension() {
        let row = JobDetailRow {
            dead_letter: Some("orders_dlq".to_owned()),
            ..detail_row("deadLetter")
        };
        let detail = to_detail(row).unwrap();
        assert_eq!(
            detail.extensions.get("deadLetter"),
            Some(&Json::from("orders_dlq"))
        );
    }

    #[test]
    fn to_detail_non_routed_row_omits_the_dead_letter_extension() {
        let detail = to_detail(detail_row("active")).unwrap();
        assert!(!detail.extensions.contains_key("deadLetter"));
    }

    #[test]
    fn to_detail_parses_data_and_defaults_missing_output_to_null() {
        let detail = to_detail(detail_row("active")).unwrap();
        assert_eq!(detail.data, serde_json::json!({ "ok": true }));
        assert_eq!(detail.output, Json::Null);
    }

    #[test]
    fn opt_str_json_wraps_some_as_a_json_string_and_none_as_null() {
        assert_eq!(
            opt_str_json(Some("sk".to_owned())),
            Json::String("sk".to_owned())
        );
        assert_eq!(opt_str_json(None), Json::Null);
    }

    #[test]
    fn to_detail_exposes_singleton_key_policy_and_priority_extension_values() {
        let row = JobDetailRow {
            singleton_key: Some("sk".to_owned()),
            policy: Some("throttle".to_owned()),
            priority: 7,
            ..detail_row("active")
        };
        let detail = to_detail(row).unwrap();
        assert_eq!(detail.extensions["singletonKey"], serde_json::json!("sk"));
        assert_eq!(detail.extensions["policy"], serde_json::json!("throttle"));
        assert_eq!(detail.extensions["priority"], serde_json::json!(7));
    }

    #[test]
    fn paginate_reports_more_when_an_extra_row_was_fetched() {
        assert_eq!(paginate(vec![1u8, 2, 3, 4], 3), (vec![1u8, 2, 3], true));
    }

    #[test]
    fn paginate_reports_no_more_at_exactly_the_limit() {
        assert_eq!(paginate(vec![1u8, 2, 3], 3), (vec![1u8, 2, 3], false));
    }

    #[test]
    fn paginate_reports_no_more_below_the_limit() {
        assert_eq!(paginate(vec![1u8, 2], 3), (vec![1u8, 2], false));
    }

    #[test]
    fn paginate_with_zero_limit_yields_empty_page_and_never_more() {
        assert_eq!(paginate(vec![1u8], 0), (Vec::<u8>::new(), false));
    }

    #[test]
    fn paginate_on_empty_input_reports_no_more() {
        assert_eq!(paginate(Vec::<u8>::new(), 3), (Vec::<u8>::new(), false));
    }

    #[test]
    fn is_uuid_rejects_a_malformed_id() {
        assert!(!is_uuid("not-a-uuid"));
    }

    #[test]
    fn is_uuid_accepts_a_well_formed_uuid() {
        assert!(is_uuid("11111111-1111-1111-1111-111111111111"));
    }
}
