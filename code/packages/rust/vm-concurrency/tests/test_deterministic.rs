//! Tests for deterministic scheduler behaviour.
//!
//! The deterministic scheduler (constructed via `Scheduler::deterministic(seed)`)
//! must satisfy the following properties that make it suitable for test suites
//! and replay-based debugging:
//!
//! 1. **FIFO ready-queue** — tasks run in the order they were spawned / enqueued.
//! 2. **Clock starts at zero** — `current_time()` returns `Deadline(0)` before
//!    any `advance_clock` call.
//! 3. **No spontaneous clock advance** — the scheduler never advances time on
//!    its own; only explicit `advance_clock` calls move the clock.
//! 4. **`advance_clock` wakes sleeping tasks** — all tasks whose deadline ≤ the
//!    new clock value are moved to the ready queue.
//! 5. **Seed is stored and retrievable** — `seed()` returns the value passed to
//!    `Scheduler::deterministic`.

use vm_concurrency::{Scheduler, Deadline, Value};

fn make_sched() -> Scheduler {
    Scheduler::deterministic(0)
}

// ---------------------------------------------------------------------------
// FIFO scheduling order
// ---------------------------------------------------------------------------

/// Spawning three tasks and picking them without any yields must return them
/// in spawn order (FIFO).
#[test]
fn deterministic_fifo_run_order() {
    let mut sched = make_sched();
    let t1 = sched.task_spawn().unwrap();
    let t2 = sched.task_spawn().unwrap();
    let t3 = sched.task_spawn().unwrap();

    assert_eq!(sched.pick_next().unwrap(), t1);
    assert_eq!(sched.pick_next().unwrap(), t2);
    assert_eq!(sched.pick_next().unwrap(), t3);
}

/// After a yield, the yielding task is re-enqueued at the back of the queue.
#[test]
fn deterministic_yield_goes_to_back_of_queue() {
    let mut sched = make_sched();
    let t1 = sched.task_spawn().unwrap();
    let t2 = sched.task_spawn().unwrap();

    // t1 runs and yields
    let first = sched.pick_next().unwrap();
    assert_eq!(first, t1);
    sched.set_current(first);
    sched.task_yield().unwrap();

    // t2 should be next, then t1 again
    assert_eq!(sched.pick_next().unwrap(), t2);
    assert_eq!(sched.pick_next().unwrap(), t1);
}

// ---------------------------------------------------------------------------
// Clock behaviour
// ---------------------------------------------------------------------------

/// The clock starts at zero nanoseconds.
#[test]
fn deterministic_clock_starts_at_zero() {
    let sched = make_sched();
    assert_eq!(sched.current_time(), Deadline::from_nanos(0));
}

/// `advance_clock` to a specific point; the scheduler's clock must reflect
/// that value.
#[test]
fn deterministic_advance_clock_sets_time() {
    let mut sched = make_sched();
    let t = Deadline::from_nanos(1_000_000);
    sched.advance_clock(t);
    assert_eq!(sched.current_time(), t);
}

/// The clock does not advance on its own between `pick_next` calls.
#[test]
fn deterministic_no_spontaneous_clock_advance() {
    let mut sched = make_sched();
    let t1 = sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);

    // Park t1 with a sleep far in the future
    let deadline = Deadline::from_nanos(999_999_999);
    sched.task_sleep(deadline).unwrap();

    // Clock has NOT been advanced; the task should still be parked
    assert!(!sched.has_ready());
    assert_eq!(sched.current_time(), Deadline::from_nanos(0));
    let _ = t1;
}

/// After `advance_clock` past a sleeping task's deadline, that task is woken.
#[test]
fn deterministic_advance_clock_wakes_sleeping_task() {
    let mut sched = make_sched();
    let t1 = sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);

    let deadline = Deadline::from_nanos(5_000_000);
    sched.task_sleep(deadline).unwrap();

    // Not ready yet
    assert!(!sched.has_ready());

    // Advance clock to exactly the deadline — task wakes
    sched.advance_clock(deadline);
    assert!(sched.has_ready());
    let next = sched.pick_next().unwrap();
    assert_eq!(next, t1);
}

/// Multiple tasks sleeping at different deadlines are woken correctly when
/// the clock is advanced past all of them.
#[test]
fn deterministic_advance_clock_wakes_multiple_sleepers() {
    let mut sched = make_sched();
    let t1 = sched.task_spawn().unwrap();
    let t2 = sched.task_spawn().unwrap();

    // t1 sleeps until 2ms
    let _n = sched.pick_next().unwrap(); // t1

    sched.set_current(_n);
    sched.task_sleep(Deadline::from_nanos(2_000_000)).unwrap();

    // t2 sleeps until 4ms
    let _n = sched.pick_next().unwrap(); // t2

    sched.set_current(_n);
    sched.task_sleep(Deadline::from_nanos(4_000_000)).unwrap();

    // Advance to 3ms — only t1 wakes
    sched.advance_clock(Deadline::from_nanos(3_000_000));
    assert!(sched.has_ready());
    let next = sched.pick_next().unwrap();
    assert_eq!(next, t1);
    assert!(!sched.has_ready()); // t2 still sleeping

    // Advance to 5ms — t2 wakes
    sched.advance_clock(Deadline::from_nanos(5_000_000));
    assert!(sched.has_ready());
    let next2 = sched.pick_next().unwrap();
    assert_eq!(next2, t2);
}

// ---------------------------------------------------------------------------
// Seed accessor
// ---------------------------------------------------------------------------

/// The seed passed to `Scheduler::deterministic` is retrievable via `seed()`.
#[test]
fn deterministic_seed_is_stored() {
    let sched = Scheduler::deterministic(42);
    assert_eq!(sched.seed(), 42);
}

/// A zero seed is also stored and returned correctly.
#[test]
fn deterministic_zero_seed_stored() {
    let sched = Scheduler::deterministic(0);
    assert_eq!(sched.seed(), 0);
}

// ---------------------------------------------------------------------------
// Completion removes task from subsequent picks
// ---------------------------------------------------------------------------

/// A completed task is not picked again.
#[test]
fn deterministic_completed_task_not_re_picked() {
    let mut sched = make_sched();
    let t1 = sched.task_spawn().unwrap();
    let _t2 = sched.task_spawn().unwrap();

    let _n = sched.pick_next().unwrap(); // t1


    sched.set_current(_n);
    sched.complete_current(Value::Int(0)).unwrap();

    // t1 should not reappear in subsequent picks
    while sched.has_ready() {
        let n = sched.pick_next().unwrap();
        assert_ne!(n, t1, "completed task should not be re-scheduled");
        sched.set_current(n);
        sched.complete_current(Value::Int(0)).unwrap();
    }
}
