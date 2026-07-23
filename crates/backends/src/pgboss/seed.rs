//! Test-only pg-boss v10 + v11 schema + fixture seeding for the integration
//! suite.
//!
//! This is the ONLY adapter module permitted to run DDL/DML: it is
//! `#[cfg(feature = "pg-integration")]` and never compiled into the read-only
//! runtime path. The v10 DDL is transcribed verbatim from pg-boss `10.1.5`
//! `src/plans.js` (schema version 24): the `job_state` enum, the `version`,
//! `queue`, and LIST-partitioned `job` tables, and the parent primary key. Each
//! v10 queue's LIST partition is created (and attached) before any of that
//! queue's jobs are inserted — the v10 `job` has no default partition (R2). The
//! v11 DDL ([`apply_schema_v11`], schema version 25, from `11.0.1` `src/plans.js`)
//! keeps the same LIST-partitioned parent but attaches a single DEFAULT
//! partition (`job_common`) that catches every queue's rows. Both flavors seed
//! the identical logical fixture ([`FIXTURE_QUEUES`]/[`FIXTURE_JOBS`]).

use sqlx::PgPool;

const SCHEMA: &str = "pgboss";

/// Apply the pg-boss v10 schema and a conformance fixture spanning all six
/// native job states, a derived dead-letter case, a waiting queue, a populated
/// non-waiting queue, and a drained queue.
pub async fn seed_v10(pool: &PgPool) -> Result<(), sqlx::Error> {
    apply_schema(pool).await?;
    seed_fixture(pool).await?;
    Ok(())
}

/// Apply the pg-boss v11 schema (schema version 25) and the SAME logical fixture
/// as [`seed_v10`], proving the flavor-agnostic read path against v11's DEFAULT
/// (`job_common`) partition layout.
pub async fn seed_v11(pool: &PgPool) -> Result<(), sqlx::Error> {
    apply_schema_v11(pool).await?;
    seed_fixture_v11(pool).await?;
    Ok(())
}

/// Create the pg-boss v10 schema objects and stamp schema version 24.
pub async fn apply_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    let ddl = [
        // gen_random_uuid() is built-in on PG13+; pgcrypto covers older images.
        "CREATE EXTENSION IF NOT EXISTS pgcrypto".to_owned(),
        format!("CREATE SCHEMA IF NOT EXISTS {SCHEMA}"),
        // ENUM declaration order IS the numeric sort order: created(0) < retry(1)
        // < active(2) < completed(3) < cancelled(4) < failed(5).
        format!(
            "CREATE TYPE {SCHEMA}.job_state AS ENUM \
             ('created', 'retry', 'active', 'completed', 'cancelled', 'failed')"
        ),
        format!(
            "CREATE TABLE {SCHEMA}.version (\
               version int primary key, \
               maintained_on timestamp with time zone, \
               cron_on timestamp with time zone, \
               monitored_on timestamp with time zone)"
        ),
        format!(
            "CREATE TABLE {SCHEMA}.queue (\
               name text, \
               policy text, \
               retry_limit int, \
               retry_delay int, \
               retry_backoff bool, \
               expire_seconds int, \
               retention_minutes int, \
               dead_letter text REFERENCES {SCHEMA}.queue (name), \
               partition_name text, \
               created_on timestamp with time zone not null default now(), \
               updated_on timestamp with time zone not null default now(), \
               PRIMARY KEY (name))"
        ),
        format!(
            "CREATE TABLE {SCHEMA}.job (\
               id uuid not null default gen_random_uuid(), \
               name text not null, \
               priority integer not null default(0), \
               data jsonb, \
               state {SCHEMA}.job_state not null default('created'), \
               retry_limit integer not null default(2), \
               retry_count integer not null default(0), \
               retry_delay integer not null default(0), \
               retry_backoff boolean not null default false, \
               start_after timestamp with time zone not null default now(), \
               started_on timestamp with time zone, \
               singleton_key text, \
               singleton_on timestamp without time zone, \
               expire_in interval not null default interval '15 minutes', \
               created_on timestamp with time zone not null default now(), \
               completed_on timestamp with time zone, \
               keep_until timestamp with time zone not null default now() + interval '14 days', \
               output jsonb, \
               dead_letter text, \
               policy text) PARTITION BY LIST (name)"
        ),
        // The partitioned parent PK; attaching a partition auto-creates its copy.
        format!("ALTER TABLE {SCHEMA}.job ADD PRIMARY KEY (name, id)"),
        format!("INSERT INTO {SCHEMA}.version (version) VALUES (24)"),
    ];
    for stmt in ddl {
        sqlx::query(&stmt).execute(pool).await?;
    }
    Ok(())
}

/// Create the pg-boss v11 schema objects and stamp schema version 25. The DDL
/// mirrors pg-boss `11.0.1` `src/plans.js`: a slim `version` table, a v11 `queue`
/// (`retention_seconds`/`deletion_seconds`/`partition`/`table_name`, no
/// `partition_name`), and the same LIST-partitioned parent `job` — but with a
/// single DEFAULT partition (`job_common`) that catches every queue's rows
/// instead of v10's per-queue partitions. Only the read-touched `job` columns
/// (plus the columns `insert_job` writes) are carried; the rest are elided.
pub async fn apply_schema_v11(pool: &PgPool) -> Result<(), sqlx::Error> {
    let ddl = [
        "CREATE EXTENSION IF NOT EXISTS pgcrypto".to_owned(),
        format!("CREATE SCHEMA IF NOT EXISTS {SCHEMA}"),
        format!(
            "CREATE TYPE {SCHEMA}.job_state AS ENUM \
             ('created', 'retry', 'active', 'completed', 'cancelled', 'failed')"
        ),
        format!(
            "CREATE TABLE {SCHEMA}.version (\
               version int primary key, \
               cron_on timestamp with time zone)"
        ),
        format!(
            "CREATE TABLE {SCHEMA}.queue (\
               name text, \
               policy text, \
               retry_limit int, \
               retry_delay int, \
               retry_backoff bool, \
               retention_seconds int, \
               deletion_seconds int, \
               dead_letter text REFERENCES {SCHEMA}.queue (name), \
               partition boolean not null default false, \
               table_name text, \
               created_on timestamp with time zone not null default now(), \
               updated_on timestamp with time zone not null default now(), \
               PRIMARY KEY (name))"
        ),
        format!(
            "CREATE TABLE {SCHEMA}.job (\
               id uuid not null default gen_random_uuid(), \
               name text not null, \
               priority integer not null default(0), \
               data jsonb, \
               state {SCHEMA}.job_state not null default('created'), \
               retry_limit integer not null default(2), \
               retry_count integer not null default(0), \
               start_after timestamp with time zone not null default now(), \
               started_on timestamp with time zone, \
               singleton_key text, \
               created_on timestamp with time zone not null default now(), \
               completed_on timestamp with time zone, \
               output jsonb, \
               dead_letter text, \
               policy text) PARTITION BY LIST (name)"
        ),
        format!("ALTER TABLE {SCHEMA}.job ADD PRIMARY KEY (name, id)"),
        // The single DEFAULT partition routes every queue's rows (v11 R2).
        format!("CREATE TABLE {SCHEMA}.job_common (LIKE {SCHEMA}.job INCLUDING DEFAULTS)"),
        format!("ALTER TABLE {SCHEMA}.job ATTACH PARTITION {SCHEMA}.job_common DEFAULT"),
        format!("INSERT INTO {SCHEMA}.version (version) VALUES (25)"),
    ];
    for stmt in ddl {
        sqlx::query(&stmt).execute(pool).await?;
    }
    Ok(())
}

/// Create a queue row plus its LIST partition (R2). `name` must be a safe
/// identifier/literal — the fixture controls it — since Postgres cannot bind an
/// identifier for `CREATE TABLE`/`ATTACH PARTITION`.
pub async fn create_queue(
    pool: &PgPool,
    name: &str,
    dead_letter: Option<&str>,
) -> Result<(), sqlx::Error> {
    let partition = format!("j_{name}");
    let literal = name.replace('\'', "''");

    sqlx::query(&format!(
        "INSERT INTO {SCHEMA}.queue \
         (name, policy, retry_limit, retry_delay, retry_backoff, partition_name, dead_letter) \
         VALUES ($1, 'standard', 2, 0, false, $2, $3)"
    ))
    .bind(name)
    .bind(&partition)
    .bind(dead_letter)
    .execute(pool)
    .await?;

    sqlx::query(&format!(
        "CREATE TABLE {SCHEMA}.{partition} (LIKE {SCHEMA}.job INCLUDING DEFAULTS)"
    ))
    .execute(pool)
    .await?;
    sqlx::query(&format!(
        "ALTER TABLE {SCHEMA}.{partition} ADD CONSTRAINT {partition}_cjc CHECK (name = '{literal}')"
    ))
    .execute(pool)
    .await?;
    sqlx::query(&format!(
        "ALTER TABLE {SCHEMA}.job ATTACH PARTITION {SCHEMA}.{partition} FOR VALUES IN ('{literal}')"
    ))
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert one fixture job. Non-state columns (`started_on`, `completed_on`,
/// `retry_count`, `output`) are derived from `state` so the row is coherent for
/// E2-2b's timeline/detail reads too. `start_after` is always `now()` — Created
/// and Retry jobs are never future-dated (R1), so a waiting queue's
/// `oldest_waiting_age` is `Some` iff it holds a waiting job.
pub async fn insert_job(
    pool: &PgPool,
    name: &str,
    state: &str,
    dead_letter: Option<&str>,
) -> Result<(), sqlx::Error> {
    let (started_on, completed_on, retry_count, output) = match state {
        "created" => ("null", "null", 0, "null"),
        "retry" => ("now() - interval '1 minute'", "null", 1, "null"),
        "active" => ("now()", "null", 0, "null"),
        "completed" => (
            "now() - interval '1 minute'",
            "now()",
            0,
            "'{\"ok\":true}'::jsonb",
        ),
        "cancelled" => ("null", "now()", 0, "null"),
        "failed" => (
            "now() - interval '1 minute'",
            "now()",
            2,
            "'{\"error\":\"boom\"}'::jsonb",
        ),
        other => panic!("unknown fixture state {other}"),
    };
    let sql = format!(
        "INSERT INTO {SCHEMA}.job \
         (name, state, dead_letter, priority, retry_count, retry_limit, \
          start_after, started_on, completed_on, data, output) \
         VALUES ($1, $2::{SCHEMA}.job_state, $3, 0, {retry_count}, 2, \
                 now(), {started_on}, {completed_on}, '{{}}'::jsonb, {output})"
    );
    sqlx::query(&sql)
        .bind(name)
        .bind(state)
        .bind(dead_letter)
        .execute(pool)
        .await?;
    Ok(())
}

/// The conformance queues in creation order: `(name, dead_letter)`. The DLQ must
/// precede the origin queue that references it (dead_letter FK). `archive_q` is
/// drained (present with no jobs). Shared by both flavors so v10 and v11 seed the
/// identical logical fixture.
const FIXTURE_QUEUES: &[(&str, Option<&str>)] = &[
    ("orders_dlq", None),
    ("orders", Some("orders_dlq")),
    ("processing", None),
    ("archive_q", None),
];

/// The conformance jobs in insertion order: `(queue, state, dead_letter)`.
/// `orders` carries all six native states plus a derived dead-letter row (a
/// `failed` origin job routed elsewhere, R4); `orders_dlq` holds the DLQ's own
/// `created` copy; `processing` is populated but has NO waiting job (oldest-
/// waiting age must be `None`). Shared by both flavors.
const FIXTURE_JOBS: &[(&str, &str, Option<&str>)] = &[
    ("orders", "created", None),
    ("orders", "retry", None),
    ("orders", "active", None),
    ("orders", "completed", None),
    ("orders", "cancelled", None),
    ("orders", "failed", None),
    ("orders", "failed", Some("orders_dlq")),
    ("orders_dlq", "created", None),
    ("processing", "active", None),
    ("processing", "completed", None),
];

async fn seed_fixture(pool: &PgPool) -> Result<(), sqlx::Error> {
    for (name, dead_letter) in FIXTURE_QUEUES {
        create_queue(pool, name, *dead_letter).await?;
    }
    for (name, state, dead_letter) in FIXTURE_JOBS {
        insert_job(pool, name, state, *dead_letter).await?;
    }
    Ok(())
}

/// Insert a v11 queue row. Matches the v11 `queue` columns; there is NO per-queue
/// partition to create or attach — every row lands in the DEFAULT `job_common`
/// partition. `name` is the fixture-controlled queue key.
pub async fn create_queue_v11(
    pool: &PgPool,
    name: &str,
    dead_letter: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(&format!(
        "INSERT INTO {SCHEMA}.queue \
         (name, policy, retry_limit, retry_delay, retry_backoff, \
          retention_seconds, deletion_seconds, dead_letter, partition, table_name) \
         VALUES ($1, 'standard', 2, 0, false, NULL, NULL, $2, false, 'job_common')"
    ))
    .bind(name)
    .bind(dead_letter)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_fixture_v11(pool: &PgPool) -> Result<(), sqlx::Error> {
    for (name, dead_letter) in FIXTURE_QUEUES {
        create_queue_v11(pool, name, *dead_letter).await?;
    }
    // `insert_job` is reused verbatim: INSERT INTO {schema}.job auto-routes to
    // the DEFAULT partition on v11 exactly as it hits the per-queue partition on
    // v10.
    for (name, state, dead_letter) in FIXTURE_JOBS {
        insert_job(pool, name, state, *dead_letter).await?;
    }
    Ok(())
}
