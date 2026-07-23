//! Testcontainer-backed conformance for `PgBossBackend` over a pg-boss **v11**
//! (schema version 25) database. The v11 read path is byte-for-byte the v10 one
//! (flavor-agnostic parent-`job` reads), so the same static suite and the same
//! fixture-shape assertions must hold. Gated behind `--features pg-integration`;
//! the CI sentinel keys on the `pg_v11_integration_sentinel` test name.
#![cfg(feature = "pg-integration")]

use qb_backends::pgboss::{seed, PgBossBackend};
use qb_core::conformance::assert_static_conformance;
use qb_core::{JobState, QueueBackend};
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
async fn pgboss_v11_backend_conforms_to_the_static_suite_and_reports_fixture_shape() {
    let (_container, pool) = boot().await;
    seed::seed_v11(&pool).await.expect("seed v11 fixture");
    let backend = PgBossBackend::new(pool);

    // The same clock-free static suite the v10 flavor passes.
    assert_static_conformance(&backend).await;

    // The identical logical fixture must surface identically through the
    // flavor-agnostic reads against the v11 layout.
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

    // Populated but with no waiting job -> None.
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
async fn pgboss_v11_test_connection_reports_the_detected_v11_flavor() {
    let (_container, pool) = boot().await;
    seed::seed_v11(&pool).await.expect("seed v11 fixture");
    let backend = PgBossBackend::new(pool);

    let info = backend
        .test_connection()
        .await
        .expect("seeded v11 is supported");
    assert!(info.healthy);
    let detail = info.detail.as_deref().unwrap_or_default();
    assert!(detail.contains("25"), "{info:?}");
    assert!(detail.contains("v11"), "{info:?}");
}

#[tokio::test]
async fn pg_v11_integration_sentinel() {
    // The CI step greps for this test name to prove the v11 PG integration test
    // actually ran (feature on + Docker present).
    let (_container, pool) = boot().await;
    seed::seed_v11(&pool).await.expect("seed v11 fixture");
    let orders_jobs: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pgboss.job WHERE name = 'orders'")
            .fetch_one(&pool)
            .await
            .expect("read a seeded row back");
    assert_eq!(orders_jobs, 7, "orders fixture must seed seven jobs");
}
