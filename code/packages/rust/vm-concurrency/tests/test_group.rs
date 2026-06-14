//! Tests for task group operations: new, spawn, join, cancel, close.

use vm_concurrency::{Scheduler, GroupPolicy, Value, ConcurrencyError};

fn make_sched() -> Scheduler {
    Scheduler::deterministic(0)
}

#[test]
fn group_new_empty() {
    let mut sched = make_sched();
    let group = sched.group_new(GroupPolicy::FailFast).unwrap();
    // spawn a main task to run group_join
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    // group_join on empty group resolves immediately
    let result = sched.group_join(group).unwrap();
    assert!(result.is_some());
    assert!(result.unwrap().is_empty());
}

#[test]
fn group_spawn_adds_child() {
    let mut sched = make_sched();
    let group = sched.group_new(GroupPolicy::FailFast).unwrap();
    let child = sched.group_spawn(group).unwrap();
    assert!(sched.task_state(child).is_some());
}

#[test]
fn group_join_parks_until_all_done() {
    let mut sched = make_sched();
    let group = sched.group_new(GroupPolicy::FailFast).unwrap();
    let _child = sched.group_spawn(group).unwrap();

    // main task parks waiting for group
    let main = sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap(); // child runs first

    sched.set_current(_n);
    sched.task_yield().unwrap();
    let _n = sched.pick_next().unwrap(); // main

    sched.set_current(_n);
    let result = sched.group_join(group).unwrap();
    assert!(result.is_none(), "should be parked");

    // child completes
    let _n = sched.pick_next().unwrap(); // child

    sched.set_current(_n);
    sched.complete_current(Value::Int(7)).unwrap();

    // main should be woken
    assert!(sched.has_ready());
    let next = sched.pick_next().unwrap();
    assert_eq!(next, main);
}

#[test]
fn group_cancel_marks_children() {
    let mut sched = make_sched();
    let group = sched.group_new(GroupPolicy::FailFast).unwrap();
    let child1 = sched.group_spawn(group).unwrap();
    let child2 = sched.group_spawn(group).unwrap();
    let main = sched.task_spawn().unwrap();
    // give main control
    for _ in 0..3 { sched.pick_next(); }
    sched.set_current(main);
    sched.group_cancel(group).unwrap();
    // Both children should have their cancel flags set
    let flag1 = {
        sched.set_current(child1);
        sched.task_check_cancel().unwrap()
    };
    let flag2 = {
        sched.set_current(child2);
        sched.task_check_cancel().unwrap()
    };
    assert!(flag1);
    assert!(flag2);
}

#[test]
fn group_close_rejects_spawn() {
    let mut sched = make_sched();
    let group = sched.group_new(GroupPolicy::FailFast).unwrap();
    sched.group_close(group).unwrap();
    let result = sched.group_spawn(group);
    assert!(matches!(result, Err(ConcurrencyError::GroupClosed(_))));
}

#[test]
fn group_collect_errors_completes() {
    let mut sched = make_sched();
    let group = sched.group_new(GroupPolicy::CollectErrors).unwrap();
    let child1 = sched.group_spawn(group).unwrap();
    let child2 = sched.group_spawn(group).unwrap();
    let main = sched.task_spawn().unwrap();

    // main parks waiting for group
    for _ in 0..3 { let _ = sched.pick_next(); }
    sched.set_current(main);
    let r = sched.group_join(group).unwrap();
    assert!(r.is_none()); // parked

    // child1 fails, child2 completes — with CollectErrors the group still resolves
    sched.set_current(child1); sched.fail_current("oops".into()).unwrap();
    sched.set_current(child2); sched.complete_current(Value::Bool(true)).unwrap();

    // main should be woken after both children finish
    assert!(sched.has_ready());
}

#[test]
fn group_detached_child_not_counted() {
    let mut sched = make_sched();
    let group = sched.group_new(GroupPolicy::FailFast).unwrap();
    let child = sched.group_spawn(group).unwrap();
    sched.task_detach(child).unwrap();

    let main = sched.task_spawn().unwrap();
    // Give control to main (skip the child)
    sched.set_current(main);
    let result = sched.group_join(group).unwrap();
    // Group has no non-detached children → immediate resolve
    assert!(result.is_some());
}

#[test]
fn group_fail_fast_cancels_siblings() {
    let mut sched = make_sched();
    let group = sched.group_new(GroupPolicy::FailFast).unwrap();
    let child1 = sched.group_spawn(group).unwrap();
    let child2 = sched.group_spawn(group).unwrap();
    sched.task_spawn().unwrap(); // main

    // child1 fails → should mark child2 for cancellation
    let _ = sched.pick_next();
    sched.set_current(child1);
    sched.fail_current("boom".into()).unwrap();

    // child2 should have cancel_flag set (checked via check_cancel)
    sched.set_current(child2);
    let flag = sched.task_check_cancel().unwrap();
    assert!(flag, "FailFast should set cancel flag on siblings");
}
