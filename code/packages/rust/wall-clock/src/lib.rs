//! # Wall-Clock — injectable time for deterministic spreadsheet recalc.
//!
//! Every spreadsheet eventually faces a function that reads "now": Excel's
//! `NOW()` and `TODAY()`, R's `Sys.time()` and `Sys.Date()`, S's `date()`.
//! If those functions reach directly into `std::time::SystemTime` then the
//! whole compute stack becomes non-deterministic — same workbook, same
//! inputs, two different recalc results because two recalcs happened at
//! two different instants.
//!
//! The fix is dependency injection: every datetime/spreadsheet function
//! that needs "now" receives a `&dyn Clock` parameter. Tests inject a
//! [`FixedClock`] at a known instant; production wires a [`SystemClock`]
//! at the binary boundary.
//!
//! This crate is intentionally tiny. It defines one trait, two
//! implementations, and the `Instant` type they exchange. No dependencies
//! beyond `core::time` and (behind a feature flag) `std::time`.
//!
//! ## Why a custom `Instant`?
//!
//! `std::time::SystemTime` is not WASM-friendly on `wasm32-unknown-unknown`
//! (it panics at runtime if the platform has no clock). It also exposes
//! a platform-dependent API and lacks the arithmetic we want
//! (`add_days`, `add_months`). Our [`Instant`] is a thin Unix-epoch f64
//! seconds value — the same representation Excel and R use internally
//! when reduced to a single number.
//!
//! ```text
//!   Instant { seconds_since_epoch: 1_700_000_000.0 }
//!            ≈ 2023-11-14T22:13:20Z
//! ```
//!
//! ## Portability bar
//!
//! Per `backend-crate-catalog.md` §1: no `std::time::SystemTime` in core
//! crates without an injected clock. This crate is the only place that
//! reads the actual system clock — and only behind the `system` feature
//! flag. WASM builds use `default-features = false` and inject a
//! [`FixedClock`] or a host-supplied clock at the boundary.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// ---------------------------------------------------------------------------
// Instant
// ---------------------------------------------------------------------------

/// A point in time expressed as seconds since the Unix epoch
/// (1970-01-01T00:00:00Z).
///
/// Stored as `f64` because that is the smallest representation that
/// round-trips through Excel's date serial, R's POSIXct, and most JSON
/// date encodings. The choice has known limitations:
///
/// - Precision drops to ~microseconds for dates after year 2200 (still
///   well below f64 mantissa exhaustion).
/// - Dates before year ~1900 lose accuracy below the millisecond.
/// - There is no leap-second representation; we follow POSIX time.
///
/// For storage formats that need higher precision (TAI, nanoseconds from
/// arbitrary epoch), wrap this type at the I/O boundary.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Instant {
    /// Seconds since 1970-01-01T00:00:00Z. Negative for dates before
    /// the Unix epoch.
    pub seconds_since_epoch: f64,
}

impl Instant {
    /// Construct from raw seconds.
    pub const fn from_secs(s: f64) -> Self {
        Self {
            seconds_since_epoch: s,
        }
    }

    /// The Unix epoch itself.
    pub const EPOCH: Instant = Instant::from_secs(0.0);

    /// Add a number of seconds. Returns a new `Instant`.
    pub fn add_secs(self, secs: f64) -> Self {
        Self::from_secs(self.seconds_since_epoch + secs)
    }

    /// Difference in seconds between two instants (`self - other`).
    pub fn duration_since(self, other: Instant) -> f64 {
        self.seconds_since_epoch - other.seconds_since_epoch
    }
}

// ---------------------------------------------------------------------------
// Clock trait
// ---------------------------------------------------------------------------

/// A source of "now" that every datetime/spreadsheet function calls into
/// instead of touching `std::time` directly.
///
/// The trait is intentionally minimal — every method except `now` has a
/// default implementation derived from it. Implementors override only
/// when they have a cheaper specialised path.
pub trait Clock {
    /// Return the current instant according to this clock.
    fn now(&self) -> Instant;
}

// ---------------------------------------------------------------------------
// FixedClock — for tests
// ---------------------------------------------------------------------------

/// A clock pinned at a single instant. Use in tests for reproducible
/// recalc results.
///
/// ```
/// use wall_clock::{Clock, FixedClock, Instant};
///
/// let clock = FixedClock::new(Instant::from_secs(1_700_000_000.0));
/// assert_eq!(clock.now().seconds_since_epoch, 1_700_000_000.0);
/// ```
#[derive(Debug, Clone)]
pub struct FixedClock {
    instant: Instant,
}

impl FixedClock {
    /// Create a clock that always returns the given instant.
    pub const fn new(instant: Instant) -> Self {
        Self { instant }
    }

    /// Convenience: pin at the Unix epoch.
    pub const fn epoch() -> Self {
        Self::new(Instant::EPOCH)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Instant {
        self.instant
    }
}

// ---------------------------------------------------------------------------
// AdvancingClock — for tests that need monotonic progress
// ---------------------------------------------------------------------------

/// A clock that ticks forward by a fixed step on every `now()` call.
/// Useful for tests that verify ordering or differential behavior under
/// time progression without depending on real wall-clock time.
///
/// ```
/// use wall_clock::{AdvancingClock, Clock, Instant};
///
/// let clock = AdvancingClock::new(Instant::EPOCH, 1.0);
/// let t0 = clock.now();
/// let t1 = clock.now();
/// assert_eq!(t1.seconds_since_epoch - t0.seconds_since_epoch, 1.0);
/// ```
#[derive(Debug)]
pub struct AdvancingClock {
    state: core::cell::Cell<f64>,
    step: f64,
}

impl AdvancingClock {
    /// Create a clock starting at `start` and advancing `step_seconds` per
    /// call. Negative steps are permitted (clock runs backward).
    pub const fn new(start: Instant, step_seconds: f64) -> Self {
        Self {
            state: core::cell::Cell::new(start.seconds_since_epoch),
            step: step_seconds,
        }
    }
}

impl Clock for AdvancingClock {
    fn now(&self) -> Instant {
        let current = self.state.get();
        self.state.set(current + self.step);
        Instant::from_secs(current)
    }
}

// ---------------------------------------------------------------------------
// SystemClock — production
// ---------------------------------------------------------------------------

/// A clock that reads the host system's real wall-clock time. Only
/// available behind the `system` feature flag (default on) because
/// `std::time::SystemTime` is not available on bare `wasm32-unknown-unknown`
/// without a host shim.
///
/// WASM consumers should disable default features and inject a
/// host-supplied clock at the JavaScript boundary instead.
#[cfg(feature = "system")]
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

#[cfg(feature = "system")]
impl SystemClock {
    /// Construct a `SystemClock`. Reads the host clock on every call.
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(feature = "system")]
impl Clock for SystemClock {
    fn now(&self) -> Instant {
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            // Fallback for systems whose clocks are pre-epoch (a real
            // hardware fault, not a logic error). We return EPOCH rather
            // than panic — caller can detect via `duration_since` if it
            // cares.
            .unwrap_or_default();
        Instant::from_secs(dur.as_secs_f64())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_arithmetic_round_trips() {
        let a = Instant::from_secs(100.0);
        let b = a.add_secs(50.5);
        assert_eq!(b.seconds_since_epoch, 150.5);
        assert_eq!(b.duration_since(a), 50.5);
    }

    #[test]
    fn instant_can_predate_epoch() {
        let pre = Instant::from_secs(-1_000.0);
        assert!(pre.seconds_since_epoch < Instant::EPOCH.seconds_since_epoch);
    }

    #[test]
    fn fixed_clock_returns_same_instant_each_call() {
        let target = Instant::from_secs(42.0);
        let clock = FixedClock::new(target);
        for _ in 0..10 {
            assert_eq!(clock.now(), target);
        }
    }

    #[test]
    fn fixed_clock_epoch_helper() {
        let clock = FixedClock::epoch();
        assert_eq!(clock.now(), Instant::EPOCH);
    }

    #[test]
    fn advancing_clock_steps_forward() {
        let clock = AdvancingClock::new(Instant::from_secs(0.0), 1.0);
        let t0 = clock.now();
        let t1 = clock.now();
        let t2 = clock.now();
        assert_eq!(t0.seconds_since_epoch, 0.0);
        assert_eq!(t1.seconds_since_epoch, 1.0);
        assert_eq!(t2.seconds_since_epoch, 2.0);
    }

    #[test]
    fn advancing_clock_supports_negative_step() {
        let clock = AdvancingClock::new(Instant::from_secs(100.0), -5.0);
        let t0 = clock.now();
        let t1 = clock.now();
        assert_eq!(t0.seconds_since_epoch, 100.0);
        assert_eq!(t1.seconds_since_epoch, 95.0);
    }

    #[test]
    fn clock_trait_object_works_for_injection() {
        // The "real" use case: a function that depends on a clock.
        fn now_secs(clock: &dyn Clock) -> f64 {
            clock.now().seconds_since_epoch
        }

        let fixed = FixedClock::new(Instant::from_secs(7.0));
        assert_eq!(now_secs(&fixed), 7.0);

        let advancing = AdvancingClock::new(Instant::from_secs(10.0), 1.0);
        assert_eq!(now_secs(&advancing), 10.0);
        assert_eq!(now_secs(&advancing), 11.0);
    }

    #[cfg(feature = "system")]
    #[test]
    fn system_clock_returns_a_recent_instant() {
        let clock = SystemClock::new();
        let t = clock.now();
        // Test relative to a "definitely in the past" anchor — Jan 1 2020.
        let jan_1_2020 = 1_577_836_800.0;
        assert!(t.seconds_since_epoch > jan_1_2020);
    }
}
