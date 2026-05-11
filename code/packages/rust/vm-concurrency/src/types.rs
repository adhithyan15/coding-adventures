//! Lightweight handle and ID types for the vm-concurrency scheduler.
//!
//! All handles are `u32` wrappers — small, `Copy`, `Hash`-able, and free of
//! lifetime annotations.  The scheduler keeps the backing data in `HashMap`s
//! keyed by these handles.
//!
//! # Generating IDs
//!
//! Each handle family has its own monotonic counter inside `Scheduler`.
//! Counters start at 1 so that 0 can serve as a "null" sentinel in external
//! code if needed (the scheduler never allocates ID 0).
//!
//! # Value
//!
//! The `Value` type that moves across task/channel boundaries is the same
//! `vm_core::value::Value` enum.  We re-export it here so the rest of
//! `vm-concurrency` imports from one place.

// Re-export vm_core's Value so all modules in this crate use one import.
pub use vm_core::value::Value;

// ---------------------------------------------------------------------------
// Handle wrappers
// ---------------------------------------------------------------------------

/// Opaque handle to a scheduled task.
///
/// Tasks are identified by a `u32` that is unique within one `Scheduler`
/// instance.  Handles are stable for the lifetime of the scheduler.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct TaskHandle(pub u32);

/// Stable identifier for a task group.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GroupId(pub u32);

/// Stable identifier for a bounded channel.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChannelId(pub u32);

/// Stable identifier for a select set (one select operation under construction).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SelectSetId(pub u32);

/// Index of one arm within a select set.
///
/// Arms are allocated sequentially starting at 0 when added to a set.  The
/// winning arm's `ArmId` is returned by `select_wait`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ArmId(pub u32);

/// Opaque cancel token.
///
/// A cancel token can be checked by its owning task via `task_check_cancel`
/// or by a select arm via `select_cancel`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CancelTokenId(pub u32);

// ---------------------------------------------------------------------------
// Deadline
// ---------------------------------------------------------------------------

/// Monotonic point in time: nanoseconds since an arbitrary epoch.
///
/// In production mode the epoch is the scheduler's creation time.
/// In deterministic mode the epoch is `Deadline::ZERO` and the clock only
/// advances when `Scheduler::advance_clock` is called.
///
/// ```
/// use vm_concurrency::types::Deadline;
/// let d = Deadline::from_nanos(1_000_000);
/// assert_eq!(d.as_nanos(), 1_000_000);
/// let later = d.plus_millis(5);
/// assert_eq!(later.as_nanos(), 6_000_000);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Deadline(pub(crate) u64);

impl Deadline {
    /// The zero deadline (beginning of time in deterministic mode).
    pub const ZERO: Deadline = Deadline(0);

    /// Construct a deadline from a nanosecond timestamp.
    pub fn from_nanos(ns: u64) -> Self {
        Deadline(ns)
    }

    /// Return the nanosecond timestamp.
    pub fn as_nanos(self) -> u64 {
        self.0
    }

    /// Return a new deadline `ms` milliseconds after `self`.
    pub fn plus_millis(self, ms: u64) -> Self {
        Deadline(self.0.saturating_add(ms.saturating_mul(1_000_000)))
    }
}

// ---------------------------------------------------------------------------
// TaskState and ParkReason
// ---------------------------------------------------------------------------

/// Lifecycle state of a scheduled task.
///
/// The state machine is:
///
/// ```text
/// New ──> Ready ──> Running ──> Completed
///                     |──> Parked(reason) ──> Ready (on wakeup)
///                     |──> CancelRequested ──> Cancelled (at next safepoint)
///                     |──> Failed
/// ```
///
/// `Detached` is an overlay flag, not a distinct state: a detached task
/// goes through the same states but its completion is not joined by a parent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskState {
    /// Allocated but not yet added to the ready queue.
    New,
    /// Runnable; queued in the ready queue.
    Ready,
    /// Currently executing on the interpreter loop.
    Running,
    /// Suspended waiting for an event.
    Parked(ParkReason),
    /// Cancellation has been requested; will be honoured at the next safepoint.
    CancelRequested,
    /// Returned a value normally.
    Completed,
    /// Raised a language runtime error or VM trap.
    Failed,
    /// Exited cooperatively through cancellation.
    Cancelled,
    /// Completion is not joined by a parent (fire and forget).
    Detached,
}

/// The reason a task is parked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParkReason {
    /// Parked by `task_yield` — will be re-queued immediately.
    Yield,
    /// Parked by `task_sleep` — waiting for the deadline to pass.
    Sleep(Deadline),
    /// Parked by `task_join` — waiting for another task to complete.
    Join(TaskHandle),
    /// Parked by `chan_send` — waiting for space in a full channel.
    ChanSend(ChannelId),
    /// Parked by `chan_recv` — waiting for a value in an empty channel.
    ChanRecv(ChannelId),
    /// Parked by `group_join` — waiting for all children to complete.
    GroupJoin(GroupId),
    /// Parked by `select_wait` — waiting for any registered arm to fire.
    Select(SelectSetId),
}

// ---------------------------------------------------------------------------
// GroupPolicy
// ---------------------------------------------------------------------------

/// Failure policy for a task group.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GroupPolicy {
    /// First child failure cancels all siblings and propagates to the
    /// group_join result.  This is the default.
    #[default]
    FailFast,
    /// All children run to completion regardless of failures.
    /// `group_join` returns an aggregated list of errors.
    CollectErrors,
    /// All children run; errors are silently swallowed.
    Supervise,
}

// ---------------------------------------------------------------------------
// SendResult / RecvResult
// ---------------------------------------------------------------------------

/// Outcome of a blocking `chan_send`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SendResult {
    /// Value was enqueued immediately.
    Sent,
    /// Channel was full; the current task has been parked.
    /// It will be woken when a receiver dequeues a value.
    Parked,
}

/// Outcome of a blocking `chan_recv`.
#[derive(Clone, PartialEq, Debug)]
pub enum RecvResult {
    /// A value was dequeued immediately.
    Received(Value),
    /// Channel was empty; the current task has been parked.
    /// It will be woken when a sender enqueues a value.
    Parked,
    /// Channel was closed and its buffer is empty.
    Closed,
}

// ---------------------------------------------------------------------------
// SelectResult
// ---------------------------------------------------------------------------

/// The result of a `select_wait` that fired.
#[derive(Clone, PartialEq, Debug)]
pub struct SelectResult {
    /// Which arm fired.
    pub arm_id: ArmId,
    /// The kind of arm that fired.
    pub kind: SelectArmKind,
    /// The value associated with the fired arm (e.g. received message, join
    /// result).  `None` for timer, cancel, default, and send arms.
    pub value: Option<Value>,
    /// Whether the event was ready, the channel was closed, etc.
    pub status: SelectStatus,
}

/// The kind of a select arm.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectArmKind {
    /// `select_recv`
    Recv,
    /// `select_send`
    Send,
    /// `select_join`
    Join,
    /// `select_timer`
    Timer,
    /// `select_cancel`
    Cancel,
    /// `select_default`
    Default,
}

/// Why/how the select arm fired.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectStatus {
    /// The event happened normally (value received, task completed, timer expired, etc.)
    Ready,
    /// A recv arm fired because the channel was closed (buffer drained).
    Closed,
    /// A cancel arm fired because the token was cancelled.
    Cancelled,
    /// A timer arm fired because the deadline passed.
    TimedOut,
    /// The default arm fired because no other arm was ready.
    Default,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_from_nanos() {
        let d = Deadline::from_nanos(12345);
        assert_eq!(d.as_nanos(), 12345);
    }

    #[test]
    fn deadline_plus_millis() {
        let d = Deadline::from_nanos(0).plus_millis(5);
        assert_eq!(d.as_nanos(), 5_000_000);
    }

    #[test]
    fn deadline_plus_millis_saturates() {
        let d = Deadline::from_nanos(u64::MAX).plus_millis(1);
        assert_eq!(d.as_nanos(), u64::MAX);
    }

    #[test]
    fn deadline_ordering() {
        assert!(Deadline::ZERO < Deadline::from_nanos(1));
        assert!(Deadline::from_nanos(100) > Deadline::from_nanos(50));
    }

    #[test]
    fn task_handle_copy_eq() {
        let h1 = TaskHandle(1);
        let h2 = h1;
        assert_eq!(h1, h2);
    }

    #[test]
    fn group_policy_default_is_fail_fast() {
        assert_eq!(GroupPolicy::default(), GroupPolicy::FailFast);
    }

    #[test]
    fn send_result_sent_ne_parked() {
        assert_ne!(SendResult::Sent, SendResult::Parked);
    }

    #[test]
    fn select_arm_kind_variants_debug() {
        let kinds = [
            SelectArmKind::Recv, SelectArmKind::Send, SelectArmKind::Join,
            SelectArmKind::Timer, SelectArmKind::Cancel, SelectArmKind::Default,
        ];
        for k in &kinds {
            assert!(!format!("{:?}", k).is_empty());
        }
    }
}
