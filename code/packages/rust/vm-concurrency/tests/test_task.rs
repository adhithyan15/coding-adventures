//! Tests for task lifecycle: spawn, yield, sleep, join, cancel, detach.

use vm_concurrency::{Scheduler, TaskState, ParkReason, Deadline, GroupPolicy, Value};

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

#[test]
fn task_spawn_assigns_new_handle() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let t2 = sched.task_spawn().unwrap();
    assert_ne!(t1, t2);
}

#[test]
fn task_spawn_state_is_ready() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    assert_eq!(sched.task_state(t1), Some(TaskState::Ready));
}

#[test]
fn task_spawn_returns_increasing_ids() {
    let mut sched = Scheduler::deterministic(0);
    let ids: Vec<_> = (0..5).map(|_| sched.task_spawn().unwrap()).collect();
    for w in ids.windows(2) {
        assert!(w[0] < w[1]);
    }
}

// ---------------------------------------------------------------------------
// Yield
// ---------------------------------------------------------------------------

#[test]
fn task_yield_moves_to_ready() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let next = sched.pick_next().unwrap();
    assert_eq!(next, t1);
    sched.set_current(next);
    sched.task_yield().unwrap();
    // After yield, the task should be back in the ready queue
    assert!(sched.has_ready());
    let next2 = sched.pick_next().unwrap();
    assert_eq!(next2, t1);
}

#[test]
fn task_yield_no_current_returns_error() {
    let mut sched = Scheduler::deterministic(0);
    let result = sched.task_yield();
    assert!(result.is_err());
}

#[test]
fn multi_task_round_robin() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let t2 = sched.task_spawn().unwrap();
    let t3 = sched.task_spawn().unwrap();

    // Run order: t1 yields, t2 yields, t3 yields, t1 again…
    let n1 = sched.pick_next().unwrap(); sched.set_current(n1); sched.task_yield().unwrap();
    let n2 = sched.pick_next().unwrap(); sched.set_current(n2); sched.task_yield().unwrap();
    let n3 = sched.pick_next().unwrap(); sched.set_current(n3); sched.task_yield().unwrap();
    let n4 = sched.pick_next().unwrap();

    assert_eq!(n1, t1);
    assert_eq!(n2, t2);
    assert_eq!(n3, t3);
    assert_eq!(n4, t1); // back to start
}

// ---------------------------------------------------------------------------
// Sleep
// ---------------------------------------------------------------------------

#[test]
fn task_sleep_parks_until_deadline() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    let deadline = Deadline::from_nanos(5_000_000); // 5ms
    sched.task_sleep(deadline).unwrap();
    // No task is ready now
    assert!(!sched.has_ready());
    assert_eq!(sched.task_state(t1), Some(TaskState::Parked(ParkReason::Sleep(deadline))));
}

#[test]
fn task_sleep_wakes_on_advance_clock() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    let deadline = Deadline::from_nanos(5_000_000);
    sched.task_sleep(deadline).unwrap();
    // Task is parked
    assert!(!sched.has_ready());
    // Advance clock past deadline
    sched.advance_clock(Deadline::from_nanos(6_000_000));
    assert!(sched.has_ready());
    let next = sched.pick_next().unwrap();
    assert_eq!(next, t1);
}

#[test]
fn task_sleep_does_not_wake_before_deadline() {
    let mut sched = Scheduler::deterministic(0);
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    sched.task_sleep(Deadline::from_nanos(10_000_000)).unwrap();
    sched.advance_clock(Deadline::from_nanos(5_000_000)); // before deadline
    assert!(!sched.has_ready());
}

// ---------------------------------------------------------------------------
// Join
// ---------------------------------------------------------------------------

#[test]
fn task_join_immediate_complete() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let _t2 = sched.task_spawn().unwrap();

    // Complete t1 first
    let _n = sched.pick_next().unwrap(); // t1

    sched.set_current(_n);
    sched.complete_current(Value::Int(42)).unwrap();
    assert_eq!(sched.task_state(t1), Some(TaskState::Completed));

    // t2 joins t1 — should get the value immediately
    let _n = sched.pick_next().unwrap(); // t2

    sched.set_current(_n);
    let result = sched.task_join(t1).unwrap();
    assert_eq!(result, Some(Value::Int(42)));
}

#[test]
fn task_join_parks_waiter() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let t2 = sched.task_spawn().unwrap();

    // t2 tries to join t1 before t1 completes
    let _ = sched.pick_next(); sched.set_current(t1); sched.task_yield().unwrap();
    let _n = sched.pick_next().unwrap(); // should be t2

    sched.set_current(_n);
    let result = sched.task_join(t1).unwrap();
    assert!(result.is_none(), "should be parked");
    assert_eq!(sched.task_state(t2), Some(TaskState::Parked(ParkReason::Join(t1))));
}

#[test]
fn task_join_wake_on_complete() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let t2 = sched.task_spawn().unwrap();

    // t2 parks waiting for t1
    let _ = sched.pick_next();
    sched.set_current(t1);
    sched.task_yield().unwrap();
    let _n = sched.pick_next().unwrap(); // t2

    sched.set_current(_n);
    sched.task_join(t1).unwrap(); // parks t2

    // Now run t1 to completion
    let _n = sched.pick_next().unwrap(); // t1 (from yield)

    sched.set_current(_n);
    sched.complete_current(Value::Int(99)).unwrap();

    // t2 should now be woken (ready)
    assert!(sched.has_ready());
    let next = sched.pick_next().unwrap();
    assert_eq!(next, t2);
}

#[test]
fn task_join_unknown_handle() {
    use vm_concurrency::{TaskHandle, ConcurrencyError};
    let mut sched = Scheduler::deterministic(0);
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    let result = sched.task_join(TaskHandle(9999));
    assert!(matches!(result, Err(ConcurrencyError::UnknownTask(_))));
}

// ---------------------------------------------------------------------------
// Cancel
// ---------------------------------------------------------------------------

#[test]
fn task_cancel_sets_flag() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let t2 = sched.task_spawn().unwrap();

    let token = sched.new_cancel_token().unwrap();

    // t1 runs first and yields — it goes back to the ready queue
    let n1 = sched.pick_next().unwrap();
    assert_eq!(n1, t1);
    sched.set_current(n1);
    sched.task_yield().unwrap();

    // t2 is now running; it cancels t1 (which is queued as Ready)
    let n2 = sched.pick_next().unwrap();
    assert_eq!(n2, t2);
    sched.set_current(n2);
    let cancelled = sched.task_cancel(t1, token).unwrap();
    assert!(cancelled);
    sched.task_yield().unwrap(); // t2 yields; queue is now [t1, t2]

    // t1 is picked next; its cancel flag is set
    let _n = sched.pick_next().unwrap(); // t1
    sched.set_current(_n);
    let flag = sched.task_check_cancel().unwrap();
    assert!(flag);
    // Flag is cleared after the first check
    let flag2 = sched.task_check_cancel().unwrap();
    assert!(!flag2);
}

#[test]
fn task_cancel_unknown_handle() {
    use vm_concurrency::{TaskHandle, ConcurrencyError};
    let mut sched = Scheduler::deterministic(0);
    let token = sched.new_cancel_token().unwrap();
    let result = sched.task_cancel(TaskHandle(9999), token);
    assert!(matches!(result, Err(ConcurrencyError::UnknownTask(_))));
}

#[test]
fn task_check_cancel_no_current_returns_error() {
    let mut sched = Scheduler::deterministic(0);
    let result = sched.task_check_cancel();
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Detach
// ---------------------------------------------------------------------------

#[test]
fn task_detach_removes_from_group() {
    let mut sched = Scheduler::deterministic(0);
    let group = sched.group_new(GroupPolicy::FailFast).unwrap();
    let child = sched.group_spawn(group).unwrap();
    sched.task_detach(child).unwrap();
    // After detach, child is no longer in the group
    // We verify by checking group_join resolves immediately (group has no children)
    sched.task_spawn().unwrap(); // main task
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    // group_join on an empty group should return immediately
    let result = sched.group_join(group).unwrap();
    assert!(result.is_some());
}

// ---------------------------------------------------------------------------
// Completion
// ---------------------------------------------------------------------------

#[test]
fn task_fail_wakes_joiner() {
    let mut sched = Scheduler::deterministic(0);
    let t1 = sched.task_spawn().unwrap();
    let t2 = sched.task_spawn().unwrap();

    // t2 parks waiting for t1
    let _ = sched.pick_next();
    sched.set_current(t1);
    sched.task_yield().unwrap();
    let _n = sched.pick_next().unwrap(); // t2

    sched.set_current(_n);
    sched.task_join(t1).unwrap();

    // t1 fails
    let _n = sched.pick_next().unwrap(); // t1 from yield

    sched.set_current(_n);
    sched.fail_current("kaboom".into()).unwrap();

    // t2 should be woken
    let next = sched.pick_next().unwrap();
    assert_eq!(next, t2);
}
