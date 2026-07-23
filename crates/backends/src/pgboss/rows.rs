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
