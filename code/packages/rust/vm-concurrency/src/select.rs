//! Select-set entry: per-select-set state stored in the scheduler's select table.
//!
//! A select set is built incrementally:
//!
//! 1. `select_new()` — allocate an empty set.
//! 2. `select_recv / select_send / select_join / select_timer / select_cancel /
//!    select_default` — add arms one by one, each gets an `ArmId`.
//! 3. `select_wait(set)` — poll all arms:
//!    - if any arm is immediately ready, return its `SelectResult` synchronously;
//!    - if a default arm is present and nothing is ready, fire the default arm;
//!    - otherwise, park the current task.
//!
//! When the current task is woken (because one arm's event fired), the arm's
//! `SelectResult` is stored in the entry's `result` field so the scheduler can
//! return it to the task.

use crate::types::{
    ArmId, ChannelId, CancelTokenId, Deadline, SelectArmKind, SelectResult, SelectStatus,
    TaskHandle, Value,
};

/// One arm in a select set.
#[derive(Debug, Clone)]
pub enum SelectArm {
    Recv  { ch: ChannelId },
    Send  { ch: ChannelId, value: Value },
    Join  { task: TaskHandle },
    Timer { deadline: Deadline },
    Cancel{ token: CancelTokenId },
    Default,
}

impl SelectArm {
    /// Return the `SelectArmKind` for this arm.
    pub fn kind(&self) -> SelectArmKind {
        match self {
            SelectArm::Recv  { .. } => SelectArmKind::Recv,
            SelectArm::Send  { .. } => SelectArmKind::Send,
            SelectArm::Join  { .. } => SelectArmKind::Join,
            SelectArm::Timer { .. } => SelectArmKind::Timer,
            SelectArm::Cancel{ .. } => SelectArmKind::Cancel,
            SelectArm::Default      => SelectArmKind::Default,
        }
    }
}

/// Per-select-set data stored in the scheduler's select table.
#[derive(Debug)]
pub struct SelectSetEntry {
    /// The arms registered on this set, in order.  Index == `ArmId(index as u32)`.
    pub arms: Vec<SelectArm>,

    /// Whether a default arm was added (fires if nothing else is ready).
    pub has_default: bool,

    /// The task currently parked waiting for this select (if any).
    pub waiter: Option<TaskHandle>,

    /// The resolved result (filled when an arm fires while a task is parked).
    pub result: Option<SelectResult>,
}

impl SelectSetEntry {
    /// Create an empty select set.
    pub fn new() -> Self {
        SelectSetEntry {
            arms: Vec::new(),
            has_default: false,
            waiter: None,
            result: None,
        }
    }

    /// Add an arm and return its `ArmId`.
    pub fn add_arm(&mut self, arm: SelectArm) -> ArmId {
        if matches!(arm, SelectArm::Default) {
            self.has_default = true;
        }
        let id = ArmId(self.arms.len() as u32);
        self.arms.push(arm);
        id
    }

    /// Store a result for the parked waiter.
    pub fn resolve(&mut self, arm_id: ArmId, kind: SelectArmKind, value: Option<Value>, status: SelectStatus) {
        self.result = Some(SelectResult {
            arm_id,
            kind,
            value,
            status,
        });
    }
}

impl Default for SelectSetEntry {
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
    use crate::types::{ChannelId, Value};

    #[test]
    fn add_recv_arm() {
        let mut set = SelectSetEntry::new();
        let id = set.add_arm(SelectArm::Recv { ch: ChannelId(1) });
        assert_eq!(id, ArmId(0));
        assert_eq!(set.arms.len(), 1);
        assert!(!set.has_default);
    }

    #[test]
    fn add_default_arm_sets_flag() {
        let mut set = SelectSetEntry::new();
        let id = set.add_arm(SelectArm::Default);
        assert_eq!(id, ArmId(0));
        assert!(set.has_default);
    }

    #[test]
    fn arm_ids_are_sequential() {
        let mut set = SelectSetEntry::new();
        let id0 = set.add_arm(SelectArm::Recv { ch: ChannelId(1) });
        let id1 = set.add_arm(SelectArm::Send { ch: ChannelId(2), value: Value::Int(0) });
        let id2 = set.add_arm(SelectArm::Default);
        assert_eq!(id0, ArmId(0));
        assert_eq!(id1, ArmId(1));
        assert_eq!(id2, ArmId(2));
    }

    #[test]
    fn resolve_stores_result() {
        let mut set = SelectSetEntry::new();
        set.add_arm(SelectArm::Recv { ch: ChannelId(5) });
        set.resolve(ArmId(0), SelectArmKind::Recv, Some(Value::Int(99)), SelectStatus::Ready);
        let r = set.result.as_ref().unwrap();
        assert_eq!(r.arm_id, ArmId(0));
        assert_eq!(r.kind, SelectArmKind::Recv);
        assert_eq!(r.status, SelectStatus::Ready);
        assert_eq!(r.value, Some(Value::Int(99)));
    }

    #[test]
    fn arm_kind_correct() {
        assert_eq!(SelectArm::Recv { ch: ChannelId(0) }.kind(), SelectArmKind::Recv);
        assert_eq!(SelectArm::Send { ch: ChannelId(0), value: Value::Bool(false) }.kind(), SelectArmKind::Send);
        assert_eq!(SelectArm::Default.kind(), SelectArmKind::Default);
        assert_eq!(
            SelectArm::Timer { deadline: Deadline::ZERO }.kind(),
            SelectArmKind::Timer
        );
    }
}
