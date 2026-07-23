//! Test-only pg-boss v10 schema + fixture seeding for the integration suite.
//!
//! This is the ONLY adapter module permitted to run DDL/DML: it is
//! `#[cfg(feature = "pg-integration")]` and never compiled into the read-only
//! runtime path. The DDL is transcribed verbatim from pg-boss `10.1.5`
//! `src/plans.js` (schema version 24): the `job_state` enum, the `version`,
//! `queue`, and LIST-partitioned `job` tables, and the parent primary key. Each
//! queue's LIST partition is created (and attached) before any of that queue's
//! jobs are inserted — `job` has no default partition (R2).

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

async fn seed_fixture(pool: &PgPool) -> Result<(), sqlx::Error> {
    // The DLQ must exist before the origin queue references it (dead_letter FK).
    create_queue(pool, "orders_dlq", None).await?;
    create_queue(pool, "orders", Some("orders_dlq")).await?;
    create_queue(pool, "processing", None).await?;
    create_queue(pool, "archive_q", None).await?; // drained: present, no jobs

    // orders: all six native states + a derived dead-letter row (a `failed`
    // origin job whose own dead_letter column routes elsewhere, R4).
    insert_job(pool, "orders", "created", None).await?;
    insert_job(pool, "orders", "retry", None).await?;
    insert_job(pool, "orders", "active", None).await?;
    insert_job(pool, "orders", "completed", None).await?;
    insert_job(pool, "orders", "cancelled", None).await?;
    insert_job(pool, "orders", "failed", None).await?; // Failed: no route
    insert_job(pool, "orders", "failed", Some("orders_dlq")).await?; // DeadLetter

    // orders_dlq: the DLQ's own `created` copy of the dead-lettered unit of work.
    insert_job(pool, "orders_dlq", "created", None).await?;

    // processing: populated but with NO waiting job -> oldest_waiting_age None.
    insert_job(pool, "processing", "active", None).await?;
    insert_job(pool, "processing", "completed", None).await?;

    Ok(())
}
