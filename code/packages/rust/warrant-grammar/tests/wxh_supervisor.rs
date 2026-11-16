//! WD-HARN1: the allocation observer's supervisor is bounded, never blocks the
//! process, and survives panics.  This is the runtime test of the generated
//! prelude: the generated WX-H harness (`tests/generated/wxh_evidence.rs`) is
//! compiled against a subject that never returns for some inputs and panics for
//! others (`tests/support/wxh_subject_stalling.rs`), and the whole suite is run
//! with a 300 ms wait bound.  It lives in its own test binary because every
//! timed-out call abandons a worker and spends the process-wide timeout budget
//! (`OBS_MAX_TIMEOUTS`), which the honest tests of `tests/wxh_evidence.rs`
//! need.  The binary finishing at all is the "never blocks exit" evidence: the
//! stalled workers are still sleeping when it ends.

use std::time::{Duration, Instant};

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxh_subject {
        include!("support/wxh_subject_stalling.rs");
    }
    include!("generated/wxh_evidence.rs");

    pub fn shorten_timeout(ms: u64) {
        OBS_TIMEOUT_MS.store(ms, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn timeout_budget() -> usize {
        OBS_MAX_TIMEOUTS
    }

    pub enum Seen {
        Finished,
        Panicked,
        TimedOut,
        RefusedStalled,
        Other,
    }

    pub fn exec(job: impl FnOnce() -> V + Send + 'static) -> Seen {
        match obs_exec(job) {
            Exec::Finished(Ok(_), _) => Seen::Finished,
            Exec::Finished(Err(_), _) => Seen::Panicked,
            Exec::TimedOut(_) => Seen::TimedOut,
            Exec::RefusedStalled => Seen::RefusedStalled,
            _ => Seen::Other,
        }
    }

    /// Take every lock the supervisor, the TAP printer and the manifest writer
    /// need; returns the number of registered tests.
    pub fn take_every_shared_lock() -> usize {
        drop(lock_or_recover(&STATS));
        drop(lock_or_recover(&OBS_STATS));
        drop(lock_or_recover(&SOURCE_SHA256));
        lock_or_recover(&META).len()
    }

    /// Poison `META` by panicking while holding it, then use it again.
    pub fn poison_then_recover() -> bool {
        let t = std::thread::spawn(|| {
            let _g = META.lock().unwrap();
            panic!("poison the lock");
        });
        let _ = t.join();
        let poisoned = META.is_poisoned();
        let _ = lock_or_recover(&META).len();
        poisoned
    }
}
use harness::Seen;

include!("support/wxh_runner.rs");

#[test]
fn a_stalled_or_panicking_subject_fails_instances_in_bounded_time() {
    std::panic::set_hook(Box::new(|_| ()));
    harness::shorten_timeout(300);

    // 1. A job that never answers is abandoned at the bound, and the same
    //    thread can still use every lock while that worker is stalled.
    let t = Instant::now();
    let seen = harness::exec(|| {
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    });
    assert!(matches!(seen, Seen::TimedOut), "a stalled call must time out");
    assert!(t.elapsed() < Duration::from_secs(30), "took {:?}", t.elapsed());
    harness::take_every_shared_lock();

    // 2. A panicking job is a reported panic, not a dead supervisor, and the
    //    next call starts a fresh worker.
    assert!(matches!(harness::exec(|| panic!("boom")), Seen::Panicked));
    assert!(matches!(harness::exec(|| harness::V::Bytes(vec![])), Seen::Finished));

    // 3. A poisoned lock is recovered, not propagated.
    assert!(harness::poison_then_recover(), "the lock must have been poisoned");
    harness::take_every_shared_lock();

    // 4. The whole generated suite, against the stalling subject, finishes in
    //    bounded time: timeouts and panics are failed verdicts with distinct
    //    messages, and once the timeout budget is spent calls are refused
    //    without waiting.  (Two timeouts are left in the budget after step 1.)
    assert!(harness::timeout_budget() >= 2);
    let t = Instant::now();
    let run = run_once();
    let elapsed = t.elapsed();
    assert!(elapsed < Duration::from_secs(120), "the suite took {elapsed:?}");
    assert!(run.failed > 0, "a stalling subject must fail the evidence");
    let messages: Vec<String> = run.outcomes.iter().filter(|o| !o.passed()).map(message).collect();
    assert!(
        messages.iter().any(|m| m.contains("observer-timeout") && m.contains("did not return within 0.3 seconds")),
        "no timeout verdict in {messages:?}"
    );
    harness::take_every_shared_lock();
}
