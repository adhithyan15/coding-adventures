//! Regression tests for the observer's own resource caps (VER04j 14.2 item 33).
//!
//! A bomb parks the worker that asked for it, and a parked worker is never
//! freed.  Two process-wide caps bound what a hostile (or merely buggy) subject
//! can make the test binary hold: the number of abandoned workers and the bytes
//! they hold.  Past either cap observed calls are *refused*, not started.
//!
//! This is its own test binary because tripping the observer deliberately uses
//! up the budget that every other observed test in a process needs.

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxh_subject {
        include!("support/wxh_subject_fixed.rs");
    }
    include!("generated/wxh_evidence.rs");

    #[derive(Debug, PartialEq, Eq)]
    pub enum Seen {
        Finished(u64),
        Panicked(u64),
        Tripped(u64),
        Refused,
    }

    pub fn exec(job: impl FnOnce() -> V + Send + 'static) -> Seen {
        match obs_exec(job) {
            Exec::Finished(Ok(_), p) => Seen::Finished(p),
            Exec::Finished(Err(_), p) => Seen::Panicked(p),
            Exec::Tripped(p) => Seen::Tripped(p),
            Exec::Refused => Seen::Refused,
        }
    }

    pub const MAX_ABANDONED: usize = OBS_MAX_ABANDONED;
    pub const MAX_HELD: u64 = OBS_MAX_HELD;
    pub const CAP: u64 = OBSERVER_CAP_BYTES;
}
use harness::{Seen, V};

/// A call that holds `hold` bytes (lazily zeroed, so not resident) and then
/// asks for more than the cap allows.
fn bomb(hold: usize) -> Seen {
    harness::exec(move || {
        let held = vec![0u8; hold];
        let more: Vec<u8> = Vec::with_capacity(harness::CAP as usize);
        V::Bool(held.len() + more.len() > 0)
    })
}

#[test]
fn abandoned_workers_and_the_bytes_they_hold_are_capped() {
    // Phase 1: every bomb holds 40 MiB, so the *bytes* cap (1 GiB) is reached
    // after 26 abandoned workers, well before the count cap (64).
    let hold = 40 << 20;
    let mut tripped = 0u64;
    let mut held = 0u64;
    let refused_after = loop {
        match bomb(hold) {
            Seen::Tripped(_) => {
                tripped += 1;
                held += hold as u64;
            }
            Seen::Refused => break tripped,
            other => panic!("{other:?}"),
        }
        assert!(tripped < harness::MAX_ABANDONED as u64, "the bytes cap never fired");
    };
    assert!(held >= harness::MAX_HELD, "refused at {held} held bytes, under the cap");
    assert!(refused_after >= 25, "refused too early ({refused_after})");
    // Once refused, a perfectly harmless call is refused as well: nothing runs.
    assert_eq!(harness::exec(|| V::Bool(true)), Seen::Refused);
    assert_eq!(harness::exec(|| V::Bool(true)), Seen::Refused);
}
