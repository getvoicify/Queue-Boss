//! Read-only SQL builders for the pg-boss v10 schema. Every string here is a
//! read-only `SELECT`; the runtime path issues no writes and no DDL (enforced by
//! the read-only grep in the E2-2a verification). The schema is interpolated
//! because Postgres cannot bind an identifier; it is the operator-configured
//! schema, defaulting to the safe literal `pgboss`.

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
        "SELECT name, \
                CASE WHEN state = 'failed' AND dead_letter IS NOT NULL AND dead_letter <> name \
                     THEN 'deadLetter' ELSE state::text END AS qb_state, \
                count(*) AS size \
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
