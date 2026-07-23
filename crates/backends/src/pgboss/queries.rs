//! Read-only SQL builders for the pg-boss v10 schema. Every string here is a
//! read-only `SELECT`; the runtime path issues no writes and no DDL (enforced by
//! the read-only grep in the E2-2a verification). The schema is interpolated
//! because Postgres cannot bind an identifier; it is the operator-configured
//! schema, defaulting to the safe literal `pgboss`.

use qb_core::{Cursor, JobFilter};

/// DeadLetter-routing projection: a `failed` row routed to another queue reads
/// as `deadLetter` everywhere (§3.4). Shared by the queue counts and the job
/// reads so both bucket identically.
const DEAD_LETTER_CASE: &str =
    "CASE WHEN state = 'failed' AND dead_letter IS NOT NULL AND dead_letter <> name \
          THEN 'deadLetter' ELSE state::text END";

/// Epoch-millisecond projection for a timestamptz column, matching the u64 ms
/// cursor so a stored/compared value round-trips exactly.
fn epoch_ms(col: &str) -> String {
    format!("(extract(epoch from {col}) * 1000)::bigint")
}

/// `SELECT version FROM <schema>.version` — the version-detect probe.
pub(crate) fn version(schema: &str) -> String {
    format!("SELECT version FROM {schema}.version")
}

/// `SELECT name FROM <schema>.queue` — the full queue set, including drained
/// queues that carry no jobs.
pub(crate) fn queue_names(schema: &str) -> String {
    format!("SELECT name FROM {schema}.queue")
}

/// Grouped per-(queue, qb_state) counts applying the §3.4 dead-letter `CASE`, so
/// a `failed` row routed to another queue is bucketed as `deadLetter`. Each job
/// lands in exactly one bucket, preserving the `total_depth` sum invariant.
pub(crate) fn state_counts(schema: &str) -> String {
    format!(
        "SELECT name, {DEAD_LETTER_CASE} AS qb_state, count(*) AS size \
         FROM {schema}.job \
         GROUP BY name, qb_state"
    )
}

/// Oldest still-waiting (Created/Retry, due) age in seconds for one queue,
/// bound as `$1`. `min(start_after)` over an empty set is `NULL`, so `age` is
/// `NULL` and maps to `None`. Future-dated backoff retries (`start_after >
/// now()`) are excluded because they are not yet due.
pub(crate) fn oldest_waiting_age(schema: &str) -> String {
    format!(
        "SELECT EXTRACT(epoch FROM now() - min(start_after))::bigint AS age \
         FROM {schema}.job \
         WHERE name = $1 AND state < 'active' AND start_after <= now()"
    )
}

/// Keyset-paginated job list applying the optional cursor + filters. Timestamps
/// project to epoch-ms (matching the u64 cursor), and the keyset compares in the
/// same ms space so the stored cursor value round-trips exactly. Placeholders
/// are numbered in the canonical bind order the runtime must mirror:
/// cursor(2) → queue(1) → states(1) → time_window(2) → search(1) → limit(1).
pub(crate) fn list_jobs(schema: &str, filter: &JobFilter, cursor: Option<&Cursor>) -> String {
    let created_ms = epoch_ms("created_on");
    let started_ms = epoch_ms("started_on");
    let completed_ms = epoch_ms("completed_on");
    let mut sql = format!(
        "SELECT {created_ms} AS created_at, id::text AS job_id, name, \
         {DEAD_LETTER_CASE} AS qb_state, {started_ms} AS started_at, \
         {completed_ms} AS completed_at, retry_count AS attempts, priority \
         FROM {schema}.job"
    );
    let mut conds: Vec<String> = Vec::new();
    let mut n = 0u32;
    if cursor.is_some() {
        let a = {
            n += 1;
            n
        };
        let b = {
            n += 1;
            n
        };
        conds.push(format!("({created_ms}, id) < (${a}, ${b}::uuid)"));
    }
    if filter.queue.is_some() {
        let p = {
            n += 1;
            n
        };
        conds.push(format!("name = ${p}"));
    }
    if filter.states.is_some() {
        let p = {
            n += 1;
            n
        };
        conds.push(format!("{DEAD_LETTER_CASE} = ANY(${p})"));
    }
    if filter.time_window.is_some() {
        let f = {
            n += 1;
            n
        };
        let t = {
            n += 1;
            n
        };
        conds.push(format!("{created_ms} >= ${f} AND {created_ms} <= ${t}"));
    }
    if filter.search.is_some() {
        let p = {
            n += 1;
            n
        };
        conds.push(format!("data::text ILIKE ${p}"));
    }
    if !conds.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conds.join(" AND "));
    }
    n += 1;
    sql.push_str(&format!(" ORDER BY created_at DESC, id DESC LIMIT ${n}"));
    sql
}

/// Single job lookup by uuid (`$1`). Extends the summary projection with the
/// retry, singleton/policy/dead-letter, and `data`/`output` columns the detail
/// view needs. `start_after` projects to epoch-ms for the `Retry` next-attempt
/// readout; `data`/`output` cast to text for in-process JSON parsing.
pub(crate) fn get_job(schema: &str) -> String {
    let created_ms = epoch_ms("created_on");
    let started_ms = epoch_ms("started_on");
    let completed_ms = epoch_ms("completed_on");
    let start_after_ms = epoch_ms("start_after");
    format!(
        "SELECT {created_ms} AS created_at, id::text AS job_id, name, \
         {DEAD_LETTER_CASE} AS qb_state, {started_ms} AS started_at, \
         {completed_ms} AS completed_at, retry_count AS attempts, priority, \
         retry_limit, {start_after_ms} AS start_after_ms, singleton_key, policy, \
         dead_letter, data::text AS data, output::text AS output \
         FROM {schema}.job WHERE id = $1::uuid LIMIT 1"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use qb_core::{JobId, JobState, TimeWindow};

    fn filter() -> JobFilter {
        JobFilter {
            queue: None,
            states: None,
            time_window: None,
            search: None,
            cursor: None,
            limit: 20,
        }
    }

    fn cursor() -> Cursor {
        Cursor {
            created_at: 1_700_000_000_000,
            id: JobId("11111111-1111-1111-1111-111111111111".to_owned()),
        }
    }

    #[test]
    fn list_jobs_without_cursor_or_filters_orders_and_limits_with_one_placeholder() {
        let sql = list_jobs("pgboss", &filter(), None);
        assert!(
            sql.contains("ORDER BY created_at DESC, id DESC LIMIT $1"),
            "{sql}"
        );
        assert!(!sql.contains(" WHERE "), "{sql}");
    }

    #[test]
    fn list_jobs_with_cursor_emits_the_ms_keyset_predicate() {
        let c = cursor();
        let sql = list_jobs("pgboss", &filter(), Some(&c));
        assert!(sql.contains("< ($1, $2::uuid)"), "{sql}");
        assert!(sql.ends_with("LIMIT $3"), "{sql}");
    }

    #[test]
    fn list_jobs_filters_by_queue_name() {
        let f = JobFilter {
            queue: Some("orders".to_owned()),
            ..filter()
        };
        let sql = list_jobs("pgboss", &f, None);
        assert!(sql.contains("name = $1"), "{sql}");
    }

    #[test]
    fn list_jobs_filters_by_states_with_any() {
        let f = JobFilter {
            states: Some(vec![JobState::Active]),
            ..filter()
        };
        let sql = list_jobs("pgboss", &f, None);
        assert!(sql.contains("= ANY($1)"), "{sql}");
    }

    #[test]
    fn list_jobs_filters_by_time_window() {
        let f = JobFilter {
            time_window: Some(TimeWindow { from: 1, to: 2 }),
            ..filter()
        };
        let sql = list_jobs("pgboss", &f, None);
        assert!(sql.contains(">= $1 AND"), "{sql}");
        assert!(sql.contains("<= $2"), "{sql}");
    }

    #[test]
    fn list_jobs_filters_by_search_ilike() {
        let f = JobFilter {
            search: Some("boom".to_owned()),
            ..filter()
        };
        let sql = list_jobs("pgboss", &f, None);
        assert!(sql.contains("data::text ILIKE $1"), "{sql}");
    }

    #[test]
    fn list_jobs_numbers_every_placeholder_in_canonical_order() {
        let c = cursor();
        let f = JobFilter {
            queue: Some("orders".to_owned()),
            states: Some(vec![JobState::Failed]),
            time_window: Some(TimeWindow { from: 1, to: 2 }),
            search: Some("boom".to_owned()),
            ..filter()
        };
        let sql = list_jobs("pgboss", &f, Some(&c));
        assert!(sql.ends_with("LIMIT $8"), "{sql}");
    }

    #[test]
    fn state_counts_reuses_the_shared_dead_letter_case() {
        let sql = state_counts("pgboss");
        assert!(sql.contains(DEAD_LETTER_CASE), "{sql}");
        assert!(sql.contains("AS qb_state"), "{sql}");
    }

    #[test]
    fn get_job_selects_a_single_row_by_uuid() {
        let sql = get_job("pgboss");
        assert!(sql.contains("WHERE id = $1::uuid LIMIT 1"), "{sql}");
        assert!(sql.contains("id::text AS job_id"), "{sql}");
    }
}
