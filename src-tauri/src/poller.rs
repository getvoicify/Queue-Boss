use std::sync::Arc;
use std::time::Duration;

use qb_core::{Clock, QueueBackend};
use tauri::ipc::Channel;

use crate::counts::QueueCounts;

/// Default poll cadence (spec §3.3): one aggregate-counts snapshot per second.
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 1000;

/// Poll `list_queues` on a fixed interval, pushing one [`QueueCounts`] snapshot
/// per tick into `channel`. The loop stops when `channel.send` errors — the
/// signal that the webview dropped the channel — so no task is leaked.
pub async fn poll_loop(
    connection_id: String,
    backend: Arc<dyn QueueBackend>,
    clock: Arc<dyn Clock>,
    channel: Channel<QueueCounts>,
    interval_ms: u64,
) {
    let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));
    loop {
        ticker.tick().await;
        match backend.list_queues().await {
            Ok(summaries) => {
                let counts =
                    QueueCounts::from_summaries(connection_id.clone(), summaries, clock.now_ms());
                if channel.send(counts).is_err() {
                    break;
                }
            }
            Err(_) => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    use async_trait::async_trait;
    use qb_core::testing::FakeBackend;
    use qb_core::{
        BackendError, BackendInfo, Capabilities, JobDetail, JobFilter, JobId, JobState, JobSummary,
        ManualClock, Page, QueueSummary,
    };
    use tauri::ipc::InvokeResponseBody;

    use super::*;

    fn capturing_channel(sink: Arc<Mutex<Vec<QueueCounts>>>) -> Channel<QueueCounts> {
        Channel::new(move |body: InvokeResponseBody| {
            match body {
                InvokeResponseBody::Json(json) => {
                    let parsed: QueueCounts = serde_json::from_str(&json).unwrap();
                    sink.lock().unwrap().push(parsed);
                }
                InvokeResponseBody::Raw(_) => panic!("expected json body"),
            }
            Ok(())
        })
    }

    #[test]
    fn default_poll_interval_is_one_second() {
        assert_eq!(DEFAULT_POLL_INTERVAL_MS, 1000);
    }

    #[tokio::test(start_paused = true)]
    async fn emits_a_stamped_snapshot_on_each_interval_tick() {
        let sink = Arc::new(Mutex::new(Vec::new()));
        let channel = capturing_channel(sink.clone());
        let backend: Arc<dyn QueueBackend> = Arc::new(FakeBackend::new());
        let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(5000));

        let handle = tokio::spawn(poll_loop(
            "sandbox".to_owned(),
            backend,
            clock,
            channel,
            1000,
        ));

        // First tick fires immediately.
        tokio::time::sleep(Duration::from_millis(1)).await;
        assert_eq!(sink.lock().unwrap().len(), 1);
        for expected in 2..=3 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            assert_eq!(sink.lock().unwrap().len(), expected);
        }

        let first = sink.lock().unwrap()[0].clone();
        assert_eq!(first.connection_id, "sandbox");
        assert_eq!(first.polled_at, 5000);
        assert_eq!(first.queues.len(), 1);
        assert_eq!(first.queues[0].queue, "default");
        assert_eq!(first.queues[0].total_depth, 3);

        handle.abort();
    }

    #[tokio::test]
    async fn stops_when_channel_send_errors() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = attempts.clone();
        let channel: Channel<QueueCounts> = Channel::new(move |_body| {
            counter.fetch_add(1, Ordering::SeqCst);
            Err(tauri::Error::FailedToReceiveMessage)
        });
        let backend: Arc<dyn QueueBackend> = Arc::new(FakeBackend::new());
        let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(0));

        let handle = tokio::spawn(poll_loop(
            "sandbox".to_owned(),
            backend,
            clock,
            channel,
            1000,
        ));

        let joined = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(joined.is_ok(), "poll loop did not stop after a send error");
        assert!(joined.unwrap().is_ok(), "poll loop task panicked");
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            1,
            "should attempt exactly one send then stop"
        );
    }

    struct FlakyBackend {
        fail_first: AtomicBool,
    }

    #[async_trait]
    impl QueueBackend for FlakyBackend {
        async fn test_connection(&self) -> Result<BackendInfo, BackendError> {
            unimplemented!()
        }

        async fn list_queues(&self) -> Result<Vec<QueueSummary>, BackendError> {
            if self.fail_first.swap(false, Ordering::SeqCst) {
                Err(BackendError::Connection("transient".to_owned()))
            } else {
                let counts = BTreeMap::from([(JobState::Active, 1u64)]);
                Ok(vec![QueueSummary::new("q", counts, None)])
            }
        }

        async fn list_jobs(&self, _filter: JobFilter) -> Result<Page<JobSummary>, BackendError> {
            unimplemented!()
        }

        async fn get_job(&self, _id: &JobId) -> Result<JobDetail, BackendError> {
            unimplemented!()
        }

        fn capabilities(&self) -> Capabilities {
            unimplemented!()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn continues_polling_after_a_backend_error() {
        let sink = Arc::new(Mutex::new(Vec::new()));
        let channel = capturing_channel(sink.clone());
        let backend: Arc<dyn QueueBackend> = Arc::new(FlakyBackend {
            fail_first: AtomicBool::new(true),
        });
        let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(7));

        let handle = tokio::spawn(poll_loop(
            "sandbox".to_owned(),
            backend,
            clock,
            channel,
            1000,
        ));

        // First tick errors: nothing emitted.
        tokio::time::sleep(Duration::from_millis(1)).await;
        assert_eq!(sink.lock().unwrap().len(), 0, "an error tick emits nothing");

        // Second tick recovers: a snapshot lands.
        tokio::time::sleep(Duration::from_millis(1000)).await;
        assert_eq!(
            sink.lock().unwrap().len(),
            1,
            "poller must keep going after a backend error"
        );

        handle.abort();
    }
}
