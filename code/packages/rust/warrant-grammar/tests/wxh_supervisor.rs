//! WD-HARN1: the allocation observer's supervisor is bounded, never blocks the
//! process, and survives panics.  This is the runtime test of the generated
//! prelude: the generated WX-H harness (`tests/generated/wxh_evidence.rs`) is
//! compiled against a subject that never returns for some inputs and panics for
//! others (`tests/support/wxh_subject_stalling.rs`), and the whole suite is run
//! with a 300 ms wait bound.  It lives in its own test binary because every
//! timed-out call abandons a worker, and a timeout in `obs_observe` is fatal for
//! the process (exit status 70 after a `Bail out!` line), so those scenarios run
//! in a child process (this binary re-run with `WXH_CHILD`).  The binary
//! finishing at all is the "never blocks exit" evidence: the stalled workers are
//! still sleeping when it ends.

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

    pub enum Seen {
        Finished,
        Panicked,
        TimedOut,
        Other,
    }

    pub fn exec(job: impl FnOnce() -> V + Send + 'static) -> Seen {
        match obs_exec(job) {
            Exec::Finished(Ok(_), _) => Seen::Finished,
            Exec::Finished(Err(_), _) => Seen::Panicked,
            Exec::TimedOut(_) => Seen::TimedOut,
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

    /// A call that never returns, observed inside the test `id` (the process
    /// must bail out; this function must not return).
    pub fn stall_in_observe(id: &str) {
        shorten_timeout(300);
        obs_begin_test(id);
        let _ = obs_observe("decompress", None, || loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        });
    }

    /// The reply slot survives poisoning: a write and a bounded wait still work.
    pub fn poisoned_slot_still_works() -> bool {
        let slot: &'static Slot = Box::leak(Box::new(Slot {
            state: Mutex::new(None),
            ready: std::sync::Condvar::new(),
        }));
        let t = std::thread::spawn(move || {
            let _g = slot.state.lock().unwrap();
            panic!("poison the slot");
        });
        let _ = t.join();
        if !slot.state.is_poisoned() {
            return false;
        }
        *lock_or_recover(&slot.state) = Some(ObsReply::Tripped(1, 2));
        let g = lock_or_recover(&slot.state);
        let (g, _) = slot
            .ready
            .wait_timeout(g, std::time::Duration::from_millis(1))
            .unwrap_or_else(|e| e.into_inner());
        matches!(*g, Some(ObsReply::Tripped(1, 2)))
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

/// Re-run this test binary as the scenario `which` and return its output.
fn run_child(which: &str) -> (std::process::Output, Duration) {
    let t = Instant::now();
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["child_scenario", "--exact", "--nocapture", "--test-threads=1"])
        .env("WXH_CHILD", which)
        .output()
        .expect("the child runs");
    (out, t.elapsed())
}

/// Only does anything in a child process (see `run_child`).
#[test]
fn child_scenario() {
    std::panic::set_hook(Box::new(|_| ()));
    match std::env::var("WXH_CHILD").as_deref() {
        // A stall inside a negative control: the verdict of a negative control
        // counts any failure as a caught fault, so this must not be a failure.
        Ok("negctl") => harness::stall_in_observe("warrant/wxh/negctl/example/decompress/missing-error"),
        // The whole generated suite against the stalling subject.
        Ok("suite") => {
            harness::shorten_timeout(300);
            let run = run_once();
            // Not reached: the run bails out at the first stalled instance.
            panic!("the suite finished with {} failures", run.failed);
        }
        _ => {}
    }
}

fn assert_bailed_out(out: &std::process::Output, elapsed: Duration) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(70), "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("Bail out! observer-timeout: `decompress` did not return within 0.3 seconds"), "{stdout}");
    assert!(stderr.contains("Bail out! observer-timeout:"), "{stderr}");
    assert!(!stdout.lines().any(|l| l.starts_with("not ok ")), "a timeout printed `not ok`: {stdout}");
    assert!(elapsed < Duration::from_secs(60), "took {elapsed:?}");
}

#[test]
fn a_stall_inside_a_negative_control_bails_out_with_status_70() {
    let (out, elapsed) = run_child("negctl");
    assert_bailed_out(&out, elapsed);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("negctl/example/decompress/missing-error"), "the bail-out names the test: {stdout}");
}

#[test]
fn a_stalling_subject_bails_out_of_the_whole_suite_in_bounded_time() {
    let (out, elapsed) = run_child("suite");
    assert_bailed_out(&out, elapsed);
}

#[test]
fn a_stalled_call_times_out_and_the_rest_of_the_harness_stays_usable() {
    std::panic::set_hook(Box::new(|_| ()));
    // 1. A job that never answers is abandoned at the bound (obs_exec itself only
    //    reports it; `obs_observe` bails out), and this thread can still take
    //    every lock while that worker is stalled.
    harness::shorten_timeout(300);
    let t = Instant::now();
    let seen = harness::exec(|| loop {
        std::thread::sleep(Duration::from_secs(1));
    });
    assert!(matches!(seen, Seen::TimedOut), "a stalled call must time out");
    assert!(t.elapsed() < Duration::from_secs(30), "took {:?}", t.elapsed());
    harness::take_every_shared_lock();

    // 2. Back to a long bound, so a slow machine cannot flake the next steps.  A
    //    panicking job is a reported panic, not a dead supervisor; the worker of
    //    step 1 was abandoned, so these calls run on a fresh one.
    harness::shorten_timeout(60_000);
    assert!(matches!(harness::exec(|| panic!("boom")), Seen::Panicked));
    assert!(matches!(harness::exec(|| harness::V::Bytes(vec![])), Seen::Finished));

    // 3. Poisoned locks are recovered, not propagated.
    assert!(harness::poison_then_recover(), "the lock must have been poisoned");
    assert!(harness::poisoned_slot_still_works(), "the reply slot must survive poisoning");
    harness::take_every_shared_lock();
}
