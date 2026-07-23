use std::sync::Arc;

use qb_backends::SandboxBackend;
use qb_core::clock::{Clock, ManualClock};
use qb_core::conformance::{assert_backend_conforms, assert_static_conformance};
use qb_core::QueueBackend;

#[tokio::test]
async fn sandbox_backend_conforms_to_the_shared_suite() {
    let clock = ManualClock::new(1_000_000);
    let backend = SandboxBackend::new(Arc::new(clock.clone()) as Arc<dyn Clock>, 0x00C0_FFEE);
    assert_backend_conforms(&backend, &clock).await;
}

#[tokio::test]
async fn sandbox_backend_conforms_to_the_static_suite() {
    // Locks the standalone clock-free entry point PgBossBackend will run.
    let clock = ManualClock::new(1_000_000);
    let backend = SandboxBackend::new(Arc::new(clock.clone()) as Arc<dyn Clock>, 0x00C0_FFEE);
    assert_static_conformance(&backend).await;
}

#[tokio::test]
async fn sandbox_backend_is_deterministic_for_same_seed_and_clock() {
    let clock = ManualClock::new(500_000);
    clock.advance(3_333);
    let a = SandboxBackend::new(Arc::new(clock.clone()) as Arc<dyn Clock>, 7);
    let b = SandboxBackend::new(Arc::new(clock.clone()) as Arc<dyn Clock>, 7);

    let filter = qb_core::JobFilter {
        queue: None,
        states: None,
        time_window: None,
        search: None,
        cursor: None,
        limit: 10_000,
    };
    let page_a = a.list_jobs(filter.clone()).await.unwrap();
    let page_b = b.list_jobs(filter).await.unwrap();
    assert_eq!(
        page_a, page_b,
        "same seed + clock must yield identical listings"
    );
}
