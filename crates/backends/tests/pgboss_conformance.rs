//! Testcontainer-backed conformance + wiring for `PgBossBackend`. Gated behind
//! `--features pg-integration` so a plain `cargo test --workspace` needs no
//! Docker. The CI sentinel keys on the `pg_integration_sentinel` test name.
#![cfg(feature = "pg-integration")]

use qb_backends::pgboss::{seed, PgBossBackend};
use qb_core::conformance::assert_queue_conformance;
use qb_core::{BackendError, JobState, QueueBackend};
use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;

async fn boot() -> (ContainerAsync<Postgres>, PgPool) {
    let container = Postgres::default()
        .start()
        .await
        .expect("start postgres container");
    let host = container.get_host().await.expect("container host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("container port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = PgPool::connect(&url).await.expect("connect to postgres");
    (container, pool)
}

#[tokio::test]
async fn pgboss_backend_conforms_to_the_queue_suite_and_reports_fixture_shape() {
    let (_container, pool) = boot().await;
    seed::seed_v10(&pool).await.expect("seed v10 fixture");
    let backend = PgBossBackend::new(pool);

    // The E2-1 queue half is the executable spec (touches only list_queues).
    assert_queue_conformance(&backend).await;

    // Explicit fixture-shape assertions pin the DeadLetter CASE and the
    // oldest-waiting predicate that the sum invariant alone cannot catch.
    let queues = backend.list_queues().await.expect("list_queues");
    let get = |name: &str| {
        queues
            .iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("queue {name} missing from {queues:?}"))
            .clone()
    };

    let orders = get("orders");
    assert_eq!(orders.counts_by_state[&JobState::Created], 1);
    assert_eq!(orders.counts_by_state[&JobState::Retry], 1);
    assert_eq!(orders.counts_by_state[&JobState::Active], 1);
    assert_eq!(orders.counts_by_state[&JobState::Completed], 1);
    assert_eq!(orders.counts_by_state[&JobState::Cancelled], 1);
    assert_eq!(orders.counts_by_state[&JobState::Failed], 1);
    assert_eq!(orders.counts_by_state[&JobState::DeadLetter], 1);
    assert_eq!(orders.total_depth, 7);
    assert!(
        orders.oldest_waiting_age.is_some(),
        "orders has due Created/Retry jobs"
    );

    let dlq = get("orders_dlq");
    assert_eq!(dlq.counts_by_state[&JobState::Created], 1);
    assert!(dlq.oldest_waiting_age.is_some());

    // Populated but with no waiting job: the age predicate must exclude the
    // active/completed rows -> None (kills `state <= 'active'` mutants).
    let processing = get("processing");
    assert_eq!(processing.counts_by_state[&JobState::Active], 1);
    assert_eq!(processing.counts_by_state[&JobState::Completed], 1);
    assert_eq!(processing.oldest_waiting_age, None);

    // Drained: present in queue but jobless.
    let archive = get("archive_q");
    assert_eq!(archive.total_depth, 0);
    assert_eq!(archive.oldest_waiting_age, None);
}

#[tokio::test]
async fn pgboss_test_connection_gates_the_schema_version() {
    let (_container, pool) = boot().await;

    // No pgboss schema yet: the version table is absent (42P01) -> Unsupported.
    let backend = PgBossBackend::new(pool.clone());
    let err = backend
        .test_connection()
        .await
        .expect_err("no schema must be Unsupported");
    assert!(matches!(err, BackendError::Unsupported(_)), "{err}");

    // Seeded at schema version 24 -> healthy v10.
    seed::seed_v10(&pool).await.expect("seed v10 fixture");
    let info = backend
        .test_connection()
        .await
        .expect("seeded v10 is supported");
    assert!(info.healthy);
    let detail = info.detail.as_deref().unwrap_or_default();
    assert!(detail.contains("24"), "{info:?}");
    assert!(detail.contains("v10"), "{info:?}");

    // Below the floor -> Unsupported with self-authored product copy.
    sqlx::query("UPDATE pgboss.version SET version = 20")
        .execute(&pool)
        .await
        .expect("downgrade version");
    let err = backend
        .test_connection()
        .await
        .expect_err("v20 is below the floor");
    match err {
        BackendError::Unsupported(msg) => assert!(msg.contains("v20"), "{msg}"),
        other => panic!("expected Unsupported, got {other}"),
    }
}

#[tokio::test]
async fn pg_integration_sentinel() {
    // The CI step greps for this test name to prove the PG integration tests
    // actually ran (feature on + Docker present).
    let (_container, pool) = boot().await;
    seed::seed_v10(&pool).await.expect("seed v10 fixture");
    let orders_jobs: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pgboss.job WHERE name = 'orders'")
            .fetch_one(&pool)
            .await
            .expect("read a seeded row back");
    assert_eq!(orders_jobs, 7, "orders fixture must seed seven jobs");
}
