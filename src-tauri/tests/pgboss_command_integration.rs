#![cfg(feature = "pg-integration")]

use std::collections::HashMap;
use std::sync::Arc;

use qb_backends::pgboss::{seed, PgBossBackend};
use qb_core::testing::FakeBackend;
use qb_core::{Clock, ManualClock, QueueBackend};
use qb_platform::InMemorySecretStore;
use queue_boss_lib::commands::{connect_impl, disconnect_impl, list_queues_impl, PgConnectConfig};
use queue_boss_lib::state::{AppState, ConnectionId};
use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;

async fn boot() -> (ContainerAsync<Postgres>, PgPool, String) {
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
    (container, pool, url)
}

fn app_state_with_sandbox() -> AppState {
    let mut backends: HashMap<ConnectionId, Arc<dyn QueueBackend>> = HashMap::new();
    backends.insert("sandbox".to_owned(), Arc::new(FakeBackend::new()));
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(0));
    AppState::new(backends, clock, Arc::new(InMemorySecretStore::new()))
}

#[tokio::test]
async fn connect_registers_pgboss_and_reads_queues_then_disconnect_tears_down() {
    let (_container, pool, url) = boot().await;
    seed::seed_v10(&pool).await.expect("seed v10 fixture");
    let state = app_state_with_sandbox();
    let config = PgConnectConfig::ConnectionString {
        connection_string: url,
    };

    let build_pool = pool.clone();
    let id = connect_impl(&state, &config, move |_cfg| {
        Ok(Arc::new(PgBossBackend::new(build_pool.clone())) as Arc<dyn QueueBackend>)
    })
    .await
    .expect("connect_impl should register the pg-boss backend");
    assert_eq!(id, "pgboss");

    let queues = list_queues_impl(&state, "pgboss")
        .await
        .expect("list_queues");
    let orders = queues
        .iter()
        .find(|q| q.name == "orders")
        .expect("orders queue present");
    assert_eq!(orders.total_depth, 7);
    let archive = queues
        .iter()
        .find(|q| q.name == "archive_q")
        .expect("archive_q present");
    assert_eq!(archive.total_depth, 0);

    disconnect_impl(&state, "pgboss").expect("disconnect");
    assert!(state.backend("pgboss").is_err());
    assert!(state.backend("sandbox").is_ok());
}

#[tokio::test]
#[allow(clippy::assertions_on_constants)]
async fn pg_integration_sentinel() {
    assert!(true);
}
