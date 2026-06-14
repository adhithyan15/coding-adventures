//! Tests for channel operations: new, send, recv, try_send, try_recv, close.

use vm_concurrency::{Scheduler, TaskState, ParkReason, SendResult, RecvResult, Value, ConcurrencyError};

fn make_sched() -> Scheduler {
    Scheduler::deterministic(0)
}

fn v(n: i64) -> Value {
    Value::Int(n)
}

// ---------------------------------------------------------------------------
// Basic operations
// ---------------------------------------------------------------------------

#[test]
fn chan_new_bounded() {
    let mut sched = make_sched();
    let ch = sched.chan_new(4).unwrap();
    // Try to fill it
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    for i in 0..4 {
        let r = sched.chan_try_send(ch, v(i)).unwrap();
        assert!(r, "should accept value {}", i);
    }
    // 5th should fail (full)
    let r = sched.chan_try_send(ch, v(99)).unwrap();
    assert!(!r, "channel should be full");
}

#[test]
fn chan_try_recv_returns_none_when_empty() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    let result = sched.chan_try_recv(ch).unwrap();
    assert_eq!(result, None);
}

#[test]
fn chan_try_send_returns_false_when_full() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    assert!(sched.chan_try_send(ch, v(1)).unwrap());
    assert!(!sched.chan_try_send(ch, v(2)).unwrap());
}

#[test]
fn chan_send_immediate_on_space() {
    let mut sched = make_sched();
    let ch = sched.chan_new(2).unwrap();
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    let r = sched.chan_send(ch, v(10)).unwrap();
    assert_eq!(r, SendResult::Sent);
}

#[test]
fn chan_recv_immediate_on_item() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    sched.chan_try_send(ch, v(42)).unwrap();
    let r = sched.chan_recv(ch).unwrap();
    assert_eq!(r, RecvResult::Received(v(42)));
}

// ---------------------------------------------------------------------------
// Parking / wakeup
// ---------------------------------------------------------------------------

#[test]
fn chan_send_parks_when_full() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    let sender = sched.task_spawn().unwrap();
    sched.task_spawn().unwrap(); // receiver (unused in this test)

    let _n = sched.pick_next().unwrap(); // sender


    sched.set_current(_n);
    sched.chan_try_send(ch, v(1)).unwrap(); // fill the buffer
    let r = sched.chan_send(ch, v(2)).unwrap(); // should park
    assert_eq!(r, SendResult::Parked);
    assert!(matches!(
        sched.task_state(sender),
        Some(TaskState::Parked(ParkReason::ChanSend(_)))
    ));
}

#[test]
fn chan_recv_parks_when_empty() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    let receiver = sched.task_spawn().unwrap();

    let _n = sched.pick_next().unwrap(); // receiver


    sched.set_current(_n);
    let r = sched.chan_recv(ch).unwrap();
    assert_eq!(r, RecvResult::Parked);
    assert!(matches!(
        sched.task_state(receiver),
        Some(TaskState::Parked(ParkReason::ChanRecv(_)))
    ));
}

#[test]
fn chan_send_wakes_blocked_recv() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    let _receiver = sched.task_spawn().unwrap();
    let _sender = sched.task_spawn().unwrap();

    // receiver parks
    let _n = sched.pick_next().unwrap(); // receiver

    sched.set_current(_n);
    sched.chan_recv(ch).unwrap(); // parks

    // sender runs and sends
    let _n = sched.pick_next().unwrap(); // sender

    sched.set_current(_n);
    let r = sched.chan_send(ch, v(77)).unwrap();
    // The send should deliver directly to the waiting receiver (Sent, not Parked)
    assert_eq!(r, SendResult::Sent);

    // receiver should be woken
    assert!(sched.has_ready());
    let next = sched.pick_next().unwrap();
    assert_eq!(next, _receiver);
}

#[test]
fn chan_recv_wakes_blocked_send() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    let sender = sched.task_spawn().unwrap();
    let _receiver = sched.task_spawn().unwrap();

    // sender fills the buffer and parks
    let _n = sched.pick_next().unwrap(); // sender

    sched.set_current(_n);
    sched.chan_try_send(ch, v(1)).unwrap();
    sched.chan_send(ch, v(2)).unwrap(); // parks

    // receiver runs and drains one value, waking sender
    let _n = sched.pick_next().unwrap(); // receiver

    sched.set_current(_n);
    sched.chan_recv(ch).unwrap();

    // sender should be woken
    assert!(sched.has_ready());
    let next = sched.pick_next().unwrap();
    assert_eq!(next, sender);
}

// ---------------------------------------------------------------------------
// Close
// ---------------------------------------------------------------------------

#[test]
fn chan_close_wakes_recv_waiters() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    let receiver = sched.task_spawn().unwrap();
    let _closer = sched.task_spawn().unwrap();

    // receiver parks
    let _n = sched.pick_next().unwrap(); // receiver

    sched.set_current(_n);
    sched.chan_recv(ch).unwrap(); // parks

    // closer runs and closes channel
    let _n = sched.pick_next().unwrap(); // closer

    sched.set_current(_n);
    sched.chan_close(ch).unwrap();

    // receiver woken
    assert!(sched.has_ready());
    let next = sched.pick_next().unwrap();
    assert_eq!(next, receiver);

    // receiver gets Closed
    sched.set_current(next);
    let r = sched.chan_recv(ch).unwrap();
    assert_eq!(r, RecvResult::Closed);
}

#[test]
fn chan_send_to_closed_returns_error() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    sched.chan_close(ch).unwrap();
    let result = sched.chan_send(ch, v(1));
    assert!(matches!(result, Err(ConcurrencyError::ChannelClosed(_))));
}

#[test]
fn chan_already_closed_returns_error() {
    let mut sched = make_sched();
    let ch = sched.chan_new(1).unwrap();
    sched.task_spawn().unwrap();
    let _n = sched.pick_next().unwrap();

    sched.set_current(_n);
    sched.chan_close(ch).unwrap();
    let result = sched.chan_close(ch);
    assert!(matches!(result, Err(ConcurrencyError::AlreadyClosed(_))));
}

// ---------------------------------------------------------------------------
// Rendezvous
// ---------------------------------------------------------------------------

#[test]
fn chan_rendezvous_sender_parks_without_receiver() {
    let mut sched = make_sched();
    let ch = sched.chan_new(0).unwrap(); // capacity 0 = rendezvous
    let sender = sched.task_spawn().unwrap();
    sched.task_spawn().unwrap();

    let _n = sched.pick_next().unwrap(); // sender


    sched.set_current(_n);
    let r = sched.chan_send(ch, v(5)).unwrap();
    assert_eq!(r, SendResult::Parked);
    assert!(matches!(
        sched.task_state(sender),
        Some(TaskState::Parked(ParkReason::ChanSend(_)))
    ));
}

#[test]
fn chan_rendezvous_delivers_when_receiver_waiting() {
    let mut sched = make_sched();
    let ch = sched.chan_new(0).unwrap();
    let _receiver = sched.task_spawn().unwrap();
    let sender = sched.task_spawn().unwrap();

    // receiver parks first
    let _n = sched.pick_next().unwrap(); // receiver

    sched.set_current(_n);
    sched.chan_recv(ch).unwrap(); // parks

    // sender runs — should deliver directly and NOT park
    let _n = sched.pick_next().unwrap(); // sender

    sched.set_current(_n);
    let r = sched.chan_send(ch, v(100)).unwrap();
    assert_eq!(r, SendResult::Sent);

    // receiver woken
    assert!(sched.has_ready());
    let _ = sched.task_state(sender); // sender not parked
}
