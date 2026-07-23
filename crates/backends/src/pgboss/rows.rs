//! `sqlx::FromRow` row structs decoded from the read-only queries. Runtime
//! `query_as` + derived `FromRow` (the sqlx `derive` feature, not `macros`)
//! keeps the build DB-free — no compile-time-checked query needs a live schema.

/// A `pgboss.version.version` integer (the schema-migration version, not the
/// pg-boss semver).
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct VersionRow {
    pub(crate) version: i32,
}

/// A queue name from `pgboss.queue`. The queue set drives drained queues (rows
/// present with no jobs) into the overview.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct QueueNameRow {
    pub(crate) name: String,
}

/// A grouped per-(queue, state) count carrying the derived `deadLetter` bucket
/// via the §3.4 `CASE` projection.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct StateCountRow {
    pub(crate) name: String,
    pub(crate) qb_state: String,
    pub(crate) size: i64,
}

/// The oldest-waiting age in whole seconds (`NULL` when nothing is due-waiting).
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct OldestAgeRow {
    pub(crate) age: Option<i64>,
}

/// A compact job row for `list_jobs`. Timestamps arrive as epoch-ms `bigint`
/// projections; `qb_state` carries the derived `deadLetter` bucket via the §3.4
/// `CASE`; `job_id` is the uuid cast to text.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct JobSummaryRow {
    pub(crate) created_at: i64,
    pub(crate) job_id: String,
    pub(crate) name: String,
    pub(crate) qb_state: String,
    pub(crate) started_at: Option<i64>,
    pub(crate) completed_at: Option<i64>,
    pub(crate) attempts: i32,
    pub(crate) priority: i32,
}

/// A full job row for `get_job`. Extends the summary projection with the retry
/// accounting, the singleton/policy/dead-letter extension columns, and the
/// `data`/`output` payloads cast to text for in-process JSON parsing.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct JobDetailRow {
    pub(crate) created_at: i64,
    pub(crate) job_id: String,
    pub(crate) name: String,
    pub(crate) qb_state: String,
    pub(crate) started_at: Option<i64>,
    pub(crate) completed_at: Option<i64>,
    pub(crate) attempts: i32,
    pub(crate) priority: i32,
    pub(crate) retry_limit: i32,
    pub(crate) start_after_ms: i64,
    pub(crate) singleton_key: Option<String>,
    pub(crate) policy: Option<String>,
    pub(crate) dead_letter: Option<String>,
    pub(crate) data: Option<String>,
    pub(crate) output: Option<String>,
}
