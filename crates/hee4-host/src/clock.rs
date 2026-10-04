//! The clock door: time is injected, never read ad hoc (AP-06).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A source of wall-clock time as a duration since the Unix epoch.
pub trait Clock: Send + Sync {
    /// Time since the Unix epoch.
    fn now(&self) -> Duration;
}

/// The real clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
    }
}

/// A settable clock for tests.
#[derive(Debug, Default)]
pub struct TestClock {
    millis: AtomicU64,
}

impl TestClock {
    /// A clock reading `millis` since the epoch.
    #[must_use]
    pub fn new(millis: u64) -> Self {
        Self {
            millis: AtomicU64::new(millis),
        }
    }

    /// Set the reading.
    pub fn set(&self, millis: u64) {
        self.millis.store(millis, Ordering::SeqCst);
    }

    /// Move the reading forward.
    pub fn advance(&self, by: Duration) {
        let step = u64::try_from(by.as_millis()).unwrap_or(u64::MAX);
        self.millis.fetch_add(step, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.millis.load(Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_is_settable() {
        let c = TestClock::new(5);
        assert_eq!(c.now(), Duration::from_millis(5));
        c.set(100);
        c.advance(Duration::from_millis(50));
        assert_eq!(c.now(), Duration::from_millis(150));
    }

    #[test]
    fn system_clock_is_after_2020() {
        assert!(SystemClock.now() > Duration::from_hours(438_288));
    }
}
