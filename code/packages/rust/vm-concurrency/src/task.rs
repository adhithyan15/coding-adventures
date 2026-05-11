//! Task entry: per-task state stored in the scheduler's task table.
//!
//! Every task has:
//!
//! - a `TaskState` (New / Ready / Running / Parked / ...).
//! - an optional name for diagnostics.
//! - a `cancel_flag`: set by `task_cancel`; observed by `task_check_cancel`
//!   and at every `is_parking` safepoint.
//! - a list of `join_waiters`: tasks blocked in `task_join` on this task.
//! - an optional `parent_group`: the task group that "owns" this task.
//! - the final `return_value` (set on completion) or `error` (set on failure).
//! - a `detached` flag (orthogonal to state: a detached task runs to completion
//!   without being joined by a parent).

use std::collections::VecDeque;

use crate::types::{GroupId, ParkReason, TaskHandle, TaskState, Value};

/// Per-task data stored in the scheduler's task table.
#[derive(Debug)]
pub struct TaskEntry {
    /// Current lifecycle state of the task.
    pub state: TaskState,

    /// Set by `task_cancel`.  Observed (and cleared) by `task_check_cancel`.
    pub cancel_flag: bool,

    /// Whether this task is fire-and-forget (no parent waits for its result).
    pub detached: bool,

    /// Optional human-readable name (language frontend may set this).
    pub name: Option<String>,

    /// The group this task belongs to, if any.
    pub parent_group: Option<GroupId>,

    /// Handles of tasks that are parked waiting for this task to complete.
    pub join_waiters: VecDeque<TaskHandle>,

    /// The value returned when the task completed normally.
    pub return_value: Option<Value>,

    /// The error string if the task failed.
    pub error: Option<String>,
}

impl TaskEntry {
    /// Create a new `TaskEntry` in the `New` state.
    pub fn new() -> Self {
        TaskEntry {
            state: TaskState::New,
            cancel_flag: false,
            detached: false,
            name: None,
            parent_group: None,
            join_waiters: VecDeque::new(),
            return_value: None,
            error: None,
        }
    }

    /// Returns `true` if the task is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled | TaskState::Detached
        )
    }

    /// Returns `true` if the task has completed (normally or by error/cancellation).
    pub fn is_done(&self) -> bool {
        self.is_terminal()
    }

    /// Mark the task as parked for the given reason.
    ///
    /// Panics if the task is not in the `Running` state — parking a non-running
    /// task is a scheduler bug.
    pub fn park(&mut self, reason: ParkReason) {
        assert_eq!(
            self.state,
            TaskState::Running,
            "can only park a Running task"
        );
        self.state = TaskState::Parked(reason);
    }

    /// Wake the task (move from Parked → Ready).
    ///
    /// Returns `true` if the state changed.  Returns `false` if the task was
    /// already not parked (e.g. cancel arrived while waking).
    pub fn wake(&mut self) -> bool {
        if matches!(self.state, TaskState::Parked(_)) {
            self.state = TaskState::Ready;
            true
        } else {
            false
        }
    }
}

impl Default for TaskEntry {
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

    #[test]
    fn new_task_is_new_state() {
        let t = TaskEntry::new();
        assert_eq!(t.state, TaskState::New);
        assert!(!t.cancel_flag);
        assert!(!t.detached);
        assert!(t.join_waiters.is_empty());
    }

    #[test]
    fn completed_is_terminal() {
        let mut t = TaskEntry::new();
        t.state = TaskState::Completed;
        assert!(t.is_terminal());
        assert!(t.is_done());
    }

    #[test]
    fn failed_is_terminal() {
        let mut t = TaskEntry::new();
        t.state = TaskState::Failed;
        assert!(t.is_terminal());
    }

    #[test]
    fn running_is_not_terminal() {
        let mut t = TaskEntry::new();
        t.state = TaskState::Running;
        assert!(!t.is_terminal());
    }

    #[test]
    fn park_running_task() {
        let mut t = TaskEntry::new();
        t.state = TaskState::Running;
        t.park(ParkReason::Yield);
        assert_eq!(t.state, TaskState::Parked(ParkReason::Yield));
    }

    #[test]
    #[should_panic(expected = "can only park a Running task")]
    fn park_non_running_panics() {
        let mut t = TaskEntry::new();
        t.park(ParkReason::Yield); // state is New, not Running
    }

    #[test]
    fn wake_parked_task() {
        let mut t = TaskEntry::new();
        t.state = TaskState::Parked(ParkReason::Yield);
        let changed = t.wake();
        assert!(changed);
        assert_eq!(t.state, TaskState::Ready);
    }

    #[test]
    fn wake_non_parked_returns_false() {
        let mut t = TaskEntry::new();
        t.state = TaskState::Ready;
        let changed = t.wake();
        assert!(!changed);
        assert_eq!(t.state, TaskState::Ready);
    }
}
