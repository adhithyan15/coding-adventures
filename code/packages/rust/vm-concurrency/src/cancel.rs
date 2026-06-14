//! Cancel token: a lightweight flag that tasks and select arms can check.
//!
//! A cancel token is:
//! - Created by `Scheduler::new_cancel_token()`.
//! - Passed to `task_cancel(target, token)` to trigger cancellation of `target`.
//! - Polled by `task_check_cancel()` which reads the current task's `cancel_flag`.
//! - Registered in a select set via `select_cancel(set, token)`.
//!
//! The cancel token entry itself only tracks whether it has been signalled.
//! The scheduler wires up the signalling: `task_cancel` sets the target task's
//! `cancel_flag` and also fires any `select_cancel` arms that are registered on
//! the token.

use crate::types::{SelectSetId, ArmId};

/// Per-cancel-token data stored in the scheduler's token table.
#[derive(Debug)]
pub struct CancelTokenEntry {
    /// Whether this token has been cancelled.
    pub cancelled: bool,

    /// Select set arms that are waiting on this token.
    /// When cancelled, the scheduler fires all of these.
    pub select_waiters: Vec<(SelectSetId, ArmId)>,
}

impl CancelTokenEntry {
    /// Create a new, uncancelled token.
    pub fn new() -> Self {
        CancelTokenEntry {
            cancelled: false,
            select_waiters: Vec::new(),
        }
    }

    /// Mark the token as cancelled.
    ///
    /// Returns the list of select waiters to wake so the scheduler can fire them.
    pub fn cancel(&mut self) -> Vec<(SelectSetId, ArmId)> {
        self.cancelled = true;
        std::mem::take(&mut self.select_waiters)
    }
}

impl Default for CancelTokenEntry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SelectSetId, ArmId};

    #[test]
    fn new_token_is_not_cancelled() {
        let t = CancelTokenEntry::new();
        assert!(!t.cancelled);
        assert!(t.select_waiters.is_empty());
    }

    #[test]
    fn cancel_sets_flag_and_returns_waiters() {
        let mut t = CancelTokenEntry::new();
        t.select_waiters.push((SelectSetId(1), ArmId(0)));
        t.select_waiters.push((SelectSetId(2), ArmId(1)));
        let waiters = t.cancel();
        assert!(t.cancelled);
        assert_eq!(waiters.len(), 2);
        assert!(t.select_waiters.is_empty()); // cleared by cancel
    }

    #[test]
    fn cancel_twice_returns_empty_waiters() {
        let mut t = CancelTokenEntry::new();
        t.cancel();
        let waiters = t.cancel();
        assert!(t.cancelled);
        assert!(waiters.is_empty());
    }
}
