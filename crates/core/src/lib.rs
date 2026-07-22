mod backend;
pub mod clock;
pub mod conformance;
mod error;
mod model;
mod page;
pub mod testing;

pub use backend::QueueBackend;
pub use clock::{Clock, ManualClock, SystemClock};
pub use error::BackendError;
pub use model::{
    oldest_waiting_age, BackendInfo, Capabilities, JobDetail, JobFilter, JobId, JobState,
    JobSummary, Json, QueueSummary, RetryReadout, Seconds, TimeWindow, TimelineEvent,
};
pub use page::{decode_cursor, encode_cursor, Cursor, Page};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_returns_crate_version() {
        assert_eq!(version(), "0.1.0");
    }
}
