//! Task group entry: per-group state stored in the scheduler's group table.
//!
//! A task group implements structured concurrency: a set of child tasks that
//! are spawned together and whose completion (or cancellation) is managed as
//! a unit.
//!
//! ## Lifecycle
//!
//! 1. `group_new(policy)` → empty, open group.
//! 2. `group_spawn(group)` → adds a child task; group tracks it in `children`.
//! 3. When a child completes/fails/cancels, the scheduler removes it from
//!    `children`; under `FailFast`, failure immediately cancels siblings.
//! 4. `group_join(group)` → parks the caller until all children are done.
//!    When the last child finishes, join waiters are woken.
//! 5. `group_close(group)` → prevents new spawns; fails if any live tasks remain
//!    (in strict mode).
//! 6. `group_cancel(group)` → sets the cancel flag on all current children.

use std::collections::{HashSet, VecDeque};

use crate::types::{GroupPolicy, TaskHandle, Value};

/// Per-group data stored in the scheduler's group table.
#[derive(Debug)]
pub struct GroupEntry {
    /// Failure handling policy.
    pub policy: GroupPolicy,

    /// Whether the group is closed to new spawns.
    pub closed: bool,

    /// Current live child tasks.  Removed when a child reaches a terminal state.
    pub children: HashSet<TaskHandle>,

    /// Errors collected from failed children (used under `CollectErrors` policy).
    pub errors: Vec<(TaskHandle, String)>,

    /// Tasks parked waiting for `group_join` to resolve.
    pub join_waiters: VecDeque<TaskHandle>,

    /// Final aggregated return values (one per child, in completion order).
    pub return_values: Vec<(TaskHandle, Value)>,
}

impl GroupEntry {
    /// Create a new empty group with the given policy.
    pub fn new(policy: GroupPolicy) -> Self {
        GroupEntry {
            policy,
            closed: false,
            children: HashSet::new(),
            errors: Vec::new(),
            join_waiters: VecDeque::new(),
            return_values: Vec::new(),
        }
    }

    /// Returns `true` if all children have reached a terminal state.
    pub fn all_done(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns `true` if the group is ready to resolve its join waiters.
    ///
    /// Under `FailFast`, this is true as soon as any error is collected AND
    /// all siblings have been cancelled (i.e. `children` is empty).
    pub fn is_resolved(&self) -> bool {
        self.children.is_empty()
    }

    /// Register a new child in the group.
    pub fn add_child(&mut self, task: TaskHandle) {
        self.children.insert(task);
    }

    /// Remove a child that has reached a terminal state.
    ///
    /// Returns `true` if the group is now fully resolved.
    pub fn remove_child(&mut self, task: TaskHandle, value: Option<Value>, error: Option<String>) -> bool {
        self.children.remove(&task);
        if let Some(v) = value {
            self.return_values.push((task, v));
        }
        if let Some(e) = error {
            self.errors.push((task, e));
        }
        self.is_resolved()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    #[test]
    fn new_group_is_empty() {
        let g = GroupEntry::new(GroupPolicy::FailFast);
        assert!(g.all_done());
        assert!(g.children.is_empty());
        assert!(!g.closed);
    }

    #[test]
    fn add_child_marks_not_done() {
        let mut g = GroupEntry::new(GroupPolicy::FailFast);
        g.add_child(TaskHandle(1));
        assert!(!g.all_done());
        assert_eq!(g.children.len(), 1);
    }

    #[test]
    fn remove_last_child_resolves_group() {
        let mut g = GroupEntry::new(GroupPolicy::FailFast);
        g.add_child(TaskHandle(1));
        let resolved = g.remove_child(TaskHandle(1), Some(Value::Int(42)), None);
        assert!(resolved);
        assert!(g.all_done());
        assert_eq!(g.return_values.len(), 1);
    }

    #[test]
    fn collect_errors_policy_stores_errors() {
        let mut g = GroupEntry::new(GroupPolicy::CollectErrors);
        g.add_child(TaskHandle(2));
        g.add_child(TaskHandle(3));
        g.remove_child(TaskHandle(2), None, Some("boom".into()));
        g.remove_child(TaskHandle(3), Some(Value::Bool(true)), None);
        assert!(g.is_resolved());
        assert_eq!(g.errors.len(), 1);
        assert_eq!(g.return_values.len(), 1);
    }

    #[test]
    fn group_is_not_resolved_with_live_children() {
        let mut g = GroupEntry::new(GroupPolicy::Supervise);
        g.add_child(TaskHandle(10));
        g.add_child(TaskHandle(11));
        g.remove_child(TaskHandle(10), None, None);
        assert!(!g.is_resolved()); // TaskHandle(11) still alive
    }
}
