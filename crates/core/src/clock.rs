//! Injectable time source. `ManualClock` drives deterministic tests;
//! `SystemClock` is wall-clock time for the running app. qb-core stays
//! dependency-clean: std only.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// A source of epoch-millisecond timestamps.
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// A test clock whose time only moves when explicitly advanced or set. Cheap to
/// clone; clones share the same underlying instant.
#[derive(Debug, Clone)]
pub struct ManualClock {
    now: Arc<AtomicU64>,
}

impl ManualClock {
    pub fn new(start_ms: u64) -> Self {
        Self {
            now: Arc::new(AtomicU64::new(start_ms)),
        }
    }

    pub fn advance(&self, delta_ms: u64) {
        self.now.fetch_add(delta_ms, Ordering::SeqCst);
    }

    pub fn set(&self, now_ms: u64) {
        self.now.store(now_ms, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }
}

/// Wall-clock time in epoch milliseconds.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_starts_at_given_instant() {
        let clock = ManualClock::new(1_000);
        assert_eq!(clock.now_ms(), 1_000);
    }

    #[test]
    fn manual_clock_advance_adds_delta() {
        let clock = ManualClock::new(1_000);
        clock.advance(250);
        assert_eq!(clock.now_ms(), 1_250);
        clock.advance(50);
        assert_eq!(clock.now_ms(), 1_300);
    }

    #[test]
    fn manual_clock_set_replaces_instant() {
        let clock = ManualClock::new(1_000);
        clock.set(42);
        assert_eq!(clock.now_ms(), 42);
    }

    #[test]
    fn manual_clock_clone_shares_instant() {
        let clock = ManualClock::new(0);
        let twin = clock.clone();
        clock.advance(500);
        assert_eq!(twin.now_ms(), 500, "a clone must observe advances");
    }

    #[test]
    fn manual_clock_is_usable_as_dyn_clock() {
        let clock: Arc<dyn Clock> = Arc::new(ManualClock::new(7));
        assert_eq!(clock.now_ms(), 7);
    }

    #[test]
    fn system_clock_returns_plausible_epoch_millis() {
        // Well after 2020-01-01T00:00:00Z in ms; kills a `-> 0` mutant.
        assert!(SystemClock.now_ms() > 1_577_836_800_000);
    }
}
