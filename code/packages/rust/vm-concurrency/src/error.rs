//! Error type for the vm-concurrency scheduler.
//!
//! All scheduler operations return `Result<T, ConcurrencyError>` when they
//! can fail due to invalid handles or violated invariants (e.g. sending to a
//! closed channel).  Internal bugs (e.g. double-completing a task) panic
//! rather than returning an error — they indicate a fault in vm-core, not in
//! user code.

use crate::types::{ChannelId, GroupId, SelectSetId, TaskHandle, CancelTokenId};

/// All ways a concurrency operation can fail.
///
/// These errors map directly onto the `UnsupportedOp` / `RuntimeError`
/// categories that `vm-core` propagates to the language runtime.
#[derive(Clone, PartialEq, Debug)]
pub enum ConcurrencyError {
    /// No current task is executing when a task-context operation was called.
    NoCurrent,

    /// `TaskHandle` refers to a task that does not exist or has been reaped.
    UnknownTask(TaskHandle),

    /// `GroupId` refers to a group that does not exist.
    UnknownGroup(GroupId),

    /// `ChannelId` refers to a channel that does not exist.
    UnknownChannel(ChannelId),

    /// `SelectSetId` refers to a set that does not exist.
    UnknownSelectSet(SelectSetId),

    /// `CancelTokenId` refers to a token that does not exist.
    UnknownCancelToken(CancelTokenId),

    /// Sending to a channel that has been closed.
    ChannelClosed(ChannelId),

    /// Closing a channel that is already closed.
    AlreadyClosed(ChannelId),

    /// Spawning a task into a group that has been closed.
    GroupClosed(GroupId),

    /// The u32 ID counter for tasks, channels, or groups has wrapped around.
    ///
    /// In practice this should never happen — 2^32 allocations per session is
    /// an extreme workload.  The error exists so the scheduler can fail
    /// gracefully rather than producing aliasing IDs.
    ResourceExhausted,
}

impl std::fmt::Display for ConcurrencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCurrent => write!(f, "no current task"),
            Self::UnknownTask(h)    => write!(f, "unknown task {:?}", h),
            Self::UnknownGroup(g)   => write!(f, "unknown group {:?}", g),
            Self::UnknownChannel(c) => write!(f, "unknown channel {:?}", c),
            Self::UnknownSelectSet(s) => write!(f, "unknown select set {:?}", s),
            Self::UnknownCancelToken(t) => write!(f, "unknown cancel token {:?}", t),
            Self::ChannelClosed(c)  => write!(f, "channel {:?} is closed", c),
            Self::AlreadyClosed(c)  => write!(f, "channel {:?} is already closed", c),
            Self::GroupClosed(g)    => write!(f, "group {:?} is closed to new spawns", g),
            Self::ResourceExhausted => write!(f, "resource exhausted (ID overflow)"),
        }
    }
}

impl std::error::Error for ConcurrencyError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChannelId, TaskHandle};

    #[test]
    fn display_no_current() {
        assert_eq!(ConcurrencyError::NoCurrent.to_string(), "no current task");
    }

    #[test]
    fn display_unknown_task() {
        let e = ConcurrencyError::UnknownTask(TaskHandle(42));
        assert!(e.to_string().contains("42"));
    }

    #[test]
    fn display_channel_closed() {
        let e = ConcurrencyError::ChannelClosed(ChannelId(7));
        assert!(e.to_string().contains("7") && e.to_string().contains("closed"));
    }

    #[test]
    fn clone_and_eq() {
        let e1 = ConcurrencyError::ResourceExhausted;
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    #[test]
    fn is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(ConcurrencyError::NoCurrent);
        assert!(!e.to_string().is_empty());
    }
}
