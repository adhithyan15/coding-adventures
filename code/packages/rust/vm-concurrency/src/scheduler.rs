//! The `Scheduler` — the central coordinator for the vm-concurrency system.
//!
//! # Design
//!
//! The `Scheduler` owns all concurrency state:
//!
//! - `tasks` — `HashMap<TaskHandle, TaskEntry>`: one entry per spawned task.
//! - `ready_queue` — `VecDeque<TaskHandle>`: FIFO queue of runnable tasks.
//! - `channels` — `HashMap<ChannelId, ChannelEntry>`: bounded message queues.
//! - `groups` — `HashMap<GroupId, GroupEntry>`: task-group scopes.
//! - `select_sets` — `HashMap<SelectSetId, SelectSetEntry>`: in-flight selects.
//! - `tokens` — `HashMap<CancelTokenId, CancelTokenEntry>`: cancel tokens.
//! - `timers` — `BinaryHeap<Reverse<(Deadline, TaskHandle)>>`: sleeping tasks.
//! - `current` — `Option<TaskHandle>`: which task is currently executing.
//!
//! # Deterministic mode
//!
//! When `Scheduler::deterministic(seed)` is used:
//!
//! - The clock starts at `Deadline::ZERO` and only advances via `advance_clock`.
//! - Ready-queue ordering is stable FIFO (tasks run in spawn order, no stealing).
//! - Select tie-breaking picks the arm with the lowest `ArmId`.
//! - The `seed` is stored for select fairness (future: shuffle arms with seeded RNG).
//!
//! # Run-loop contract
//!
//! ```text
//! loop {
//!     match scheduler.pick_next() {
//!         None => {
//!             if scheduler.is_done() { break; }
//!             scheduler.advance_clock(next_deadline);  // deterministic only
//!             continue;
//!         }
//!         Some(task) => {
//!             scheduler.set_current(task);
//!             // ... execute task instructions until park/complete/fail ...
//!         }
//!     }
//! }
//! ```

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Reverse;

use crate::cancel::CancelTokenEntry;
use crate::channel::{ChannelEntry, TryDequeueResult, TryEnqueueResult};
use crate::error::ConcurrencyError;
use crate::group::GroupEntry;
use crate::select::{SelectArm, SelectSetEntry};
use crate::task::TaskEntry;
use crate::types::{
    ArmId, CancelTokenId, ChannelId, Deadline, GroupId, GroupPolicy,
    ParkReason, RecvResult, SelectArmKind, SelectResult, SelectSetId,
    SelectStatus, SendResult, TaskHandle, TaskState, Value,
};

/// The central cooperative scheduler.
///
/// Create one per program execution.  The scheduler is single-threaded;
/// all methods take `&mut self`.
pub struct Scheduler {
    // ── Storage tables ────────────────────────────────────────────────────────
    tasks:       HashMap<TaskHandle, TaskEntry>,
    channels:    HashMap<ChannelId,  ChannelEntry>,
    groups:      HashMap<GroupId,    GroupEntry>,
    select_sets: HashMap<SelectSetId, SelectSetEntry>,
    tokens:      HashMap<CancelTokenId, CancelTokenEntry>,

    // ── Ready queue ───────────────────────────────────────────────────────────
    ready_queue: VecDeque<TaskHandle>,

    // ── Timer heap (min-heap by deadline) ─────────────────────────────────────
    /// Each entry is `Reverse((Deadline, TaskHandle))` so the smallest
    /// deadline is at the top.
    timers: BinaryHeap<Reverse<(Deadline, TaskHandle)>>,

    // ── Execution state ───────────────────────────────────────────────────────
    current: Option<TaskHandle>,

    // ── ID counters ───────────────────────────────────────────────────────────
    next_task:    u32,
    next_channel: u32,
    next_group:   u32,
    next_select:  u32,
    next_token:   u32,

    // ── Scheduler mode ────────────────────────────────────────────────────────
    deterministic: bool,
    #[allow(dead_code)]
    seed: u64,
    clock: Deadline,
}

impl Scheduler {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create a new scheduler in production (non-deterministic) mode.
    pub fn new() -> Self {
        Scheduler::_make(false, 0)
    }

    /// Create a scheduler in deterministic mode.
    ///
    /// The virtual clock starts at `Deadline::ZERO`.  Time only advances
    /// when `advance_clock` is called.
    pub fn deterministic(seed: u64) -> Self {
        Scheduler::_make(true, seed)
    }

    fn _make(deterministic: bool, seed: u64) -> Self {
        Scheduler {
            tasks: HashMap::new(),
            channels: HashMap::new(),
            groups: HashMap::new(),
            select_sets: HashMap::new(),
            tokens: HashMap::new(),
            ready_queue: VecDeque::new(),
            timers: BinaryHeap::new(),
            current: None,
            next_task: 1,
            next_channel: 1,
            next_group: 1,
            next_select: 1,
            next_token: 1,
            deterministic,
            seed,
            clock: Deadline::ZERO,
        }
    }

    // ── ID allocation helpers ─────────────────────────────────────────────────

    fn alloc_task(&mut self) -> Result<TaskHandle, ConcurrencyError> {
        let id = self.next_task;
        self.next_task = id.checked_add(1).ok_or(ConcurrencyError::ResourceExhausted)?;
        Ok(TaskHandle(id))
    }

    fn alloc_channel(&mut self) -> Result<ChannelId, ConcurrencyError> {
        let id = self.next_channel;
        self.next_channel = id.checked_add(1).ok_or(ConcurrencyError::ResourceExhausted)?;
        Ok(ChannelId(id))
    }

    fn alloc_group(&mut self) -> Result<GroupId, ConcurrencyError> {
        let id = self.next_group;
        self.next_group = id.checked_add(1).ok_or(ConcurrencyError::ResourceExhausted)?;
        Ok(GroupId(id))
    }

    fn alloc_select(&mut self) -> Result<SelectSetId, ConcurrencyError> {
        let id = self.next_select;
        self.next_select = id.checked_add(1).ok_or(ConcurrencyError::ResourceExhausted)?;
        Ok(SelectSetId(id))
    }

    fn alloc_token(&mut self) -> Result<CancelTokenId, ConcurrencyError> {
        let id = self.next_token;
        self.next_token = id.checked_add(1).ok_or(ConcurrencyError::ResourceExhausted)?;
        Ok(CancelTokenId(id))
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn require_current(&self) -> Result<TaskHandle, ConcurrencyError> {
        self.current.ok_or(ConcurrencyError::NoCurrent)
    }

    fn get_task(&self, h: TaskHandle) -> Result<&TaskEntry, ConcurrencyError> {
        self.tasks.get(&h).ok_or(ConcurrencyError::UnknownTask(h))
    }

    fn get_task_mut(&mut self, h: TaskHandle) -> Result<&mut TaskEntry, ConcurrencyError> {
        self.tasks.get_mut(&h).ok_or(ConcurrencyError::UnknownTask(h))
    }

    fn get_channel_mut(&mut self, c: ChannelId) -> Result<&mut ChannelEntry, ConcurrencyError> {
        self.channels.get_mut(&c).ok_or(ConcurrencyError::UnknownChannel(c))
    }

    fn get_group_mut(&mut self, g: GroupId) -> Result<&mut GroupEntry, ConcurrencyError> {
        self.groups.get_mut(&g).ok_or(ConcurrencyError::UnknownGroup(g))
    }

    fn get_select_mut(&mut self, s: SelectSetId) -> Result<&mut SelectSetEntry, ConcurrencyError> {
        self.select_sets.get_mut(&s).ok_or(ConcurrencyError::UnknownSelectSet(s))
    }

    fn enqueue_ready(&mut self, h: TaskHandle) {
        if let Some(task) = self.tasks.get_mut(&h) {
            task.state = TaskState::Ready;
        }
        self.ready_queue.push_back(h);
    }

    /// Park the current task and dequeue it.
    fn park_current(&mut self, reason: ParkReason) -> Result<(), ConcurrencyError> {
        let current = self.require_current()?;
        let task = self.get_task_mut(current)?;
        task.park(reason);
        self.current = None;
        Ok(())
    }

    // ── Task operations ───────────────────────────────────────────────────────

    /// Spawn a new task.  Starts in `Ready` state and is added to the ready queue.
    ///
    /// The caller is responsible for pushing an initial interpreter frame onto
    /// the task before it can execute any instructions.
    pub fn task_spawn(&mut self) -> Result<TaskHandle, ConcurrencyError> {
        let handle = self.alloc_task()?;
        let mut entry = TaskEntry::new();
        entry.state = TaskState::Ready;
        self.tasks.insert(handle, entry);
        self.ready_queue.push_back(handle);
        Ok(handle)
    }

    /// Get the handle of the currently-executing task.
    pub fn task_current(&self) -> Option<TaskHandle> {
        self.current
    }

    /// Cooperatively yield: move the current task to the back of the ready queue.
    ///
    /// This is a cooperative multitasking "give other tasks a turn" operation.
    /// The current task is immediately re-queued in `Ready` state, so it will
    /// run again once every other ready task has had a turn.
    pub fn task_yield(&mut self) -> Result<(), ConcurrencyError> {
        let current = self.require_current()?;
        // Check for pending cancellation first (yield is a safepoint).
        {
            let task = self.get_task_mut(current)?;
            if task.cancel_flag {
                task.cancel_flag = false;
                task.state = TaskState::Cancelled;
                self.current = None;
                return Ok(());
            }
            task.state = TaskState::Ready;
        }
        self.ready_queue.push_back(current);
        self.current = None;
        Ok(())
    }

    /// Park the current task until `deadline`.
    pub fn task_sleep(&mut self, deadline: Deadline) -> Result<(), ConcurrencyError> {
        let current = self.require_current()?;
        // Check for pending cancellation first.
        {
            let task = self.get_task_mut(current)?;
            if task.cancel_flag {
                task.cancel_flag = false;
                task.state = TaskState::Cancelled;
                self.current = None;
                return Ok(());
            }
        }
        self.park_current(ParkReason::Sleep(deadline))?;
        self.timers.push(Reverse((deadline, current)));
        Ok(())
    }

    /// Join another task.
    ///
    /// If the target is already completed, returns `Some(value)` immediately.
    /// If the target is failed, returns the error as a `String` value.
    /// Otherwise parks the current task; returns `None` (caller must switch tasks).
    pub fn task_join(&mut self, target: TaskHandle) -> Result<Option<Value>, ConcurrencyError> {
        let target_state = {
            let t = self.get_task(target)?;
            (t.state, t.return_value.clone(), t.error.clone())
        };

        match target_state {
            (TaskState::Completed, Some(v), _) => return Ok(Some(v)),
            (TaskState::Completed, None, _)     => return Ok(Some(Value::Null)),
            (TaskState::Failed, _, Some(e))     => return Ok(Some(Value::Str(e))),
            (TaskState::Cancelled, _, _)        => return Ok(Some(Value::Null)),
            _ => {}
        }

        // Register current task as a join waiter on the target.
        let current = self.require_current()?;
        {
            let target_entry = self.get_task_mut(target)?;
            target_entry.join_waiters.push_back(current);
        }
        self.park_current(ParkReason::Join(target))?;
        Ok(None) // caller must pick_next
    }

    /// Request cooperative cancellation of `target`.
    ///
    /// Sets the cancel flag on the target.  If the target is parked at a
    /// parking point, wake it immediately (it will observe the flag on next
    /// `task_check_cancel` or `task_yield` call).
    pub fn task_cancel(&mut self, target: TaskHandle, _token: CancelTokenId) -> Result<bool, ConcurrencyError> {
        // Verify token exists (but we don't use it yet — full token integration in LANG28C).
        let task = self.get_task_mut(target)?;
        if task.is_terminal() {
            return Ok(false);
        }
        task.cancel_flag = true;
        // Only wake the task if it is currently parked — avoid double-enqueueing
        // a task that is already in the Ready queue.  We check the state once,
        // atomically, before any mutation so the guard and the wake use the same
        // snapshot of the state.
        let was_parked = matches!(task.state, TaskState::Parked(_));
        if was_parked {
            task.state = TaskState::CancelRequested;
            // enqueue_ready transitions back to Ready and pushes to the queue.
            self.enqueue_ready(target);
        }
        Ok(true)
    }

    /// Check whether the current task's cancel flag is set, and clear it.
    ///
    /// Returns `true` if the task was cancelled (and the flag was cleared).
    pub fn task_check_cancel(&mut self) -> Result<bool, ConcurrencyError> {
        let current = self.require_current()?;
        let task = self.get_task_mut(current)?;
        if task.cancel_flag {
            task.cancel_flag = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Detach `target` from its parent group (if any).
    pub fn task_detach(&mut self, target: TaskHandle) -> Result<(), ConcurrencyError> {
        let parent_group = {
            let task = self.get_task_mut(target)?;
            let pg = task.parent_group.take();
            task.detached = true;
            pg
        };
        if let Some(group_id) = parent_group {
            if let Some(group) = self.groups.get_mut(&group_id) {
                group.children.remove(&target);
            }
        }
        Ok(())
    }

    // ── Task group operations ─────────────────────────────────────────────────

    /// Create a new empty task group.
    pub fn group_new(&mut self, policy: GroupPolicy) -> Result<GroupId, ConcurrencyError> {
        let id = self.alloc_group()?;
        self.groups.insert(id, GroupEntry::new(policy));
        Ok(id)
    }

    /// Spawn a new task inside `group`.
    pub fn group_spawn(&mut self, group: GroupId) -> Result<TaskHandle, ConcurrencyError> {
        // Verify group exists and is open.
        {
            let g = self.get_group_mut(group)?;
            if g.closed {
                return Err(ConcurrencyError::GroupClosed(group));
            }
        }
        let handle = self.alloc_task()?;
        let mut entry = TaskEntry::new();
        entry.state = TaskState::Ready;
        entry.parent_group = Some(group);
        self.tasks.insert(handle, entry);
        self.ready_queue.push_back(handle);
        self.get_group_mut(group)?.add_child(handle);
        Ok(handle)
    }

    /// Wait for all tasks in `group` to complete.
    ///
    /// Returns `Some(values)` immediately if the group is already empty.
    /// Returns `None` if the current task was parked.
    pub fn group_join(&mut self, group: GroupId) -> Result<Option<Vec<Value>>, ConcurrencyError> {
        let all_done = {
            let g = self.groups.get(&group).ok_or(ConcurrencyError::UnknownGroup(group))?;
            g.all_done()
        };
        if all_done {
            let values: Vec<Value> = self.groups[&group]
                .return_values
                .iter()
                .map(|(_, v)| v.clone())
                .collect();
            return Ok(Some(values));
        }
        let current = self.require_current()?;
        self.get_group_mut(group)?.join_waiters.push_back(current);
        self.park_current(ParkReason::GroupJoin(group))?;
        Ok(None)
    }

    /// Cancel all current children of the group.
    pub fn group_cancel(&mut self, group: GroupId) -> Result<(), ConcurrencyError> {
        let children: Vec<TaskHandle> = {
            let g = self.groups.get(&group).ok_or(ConcurrencyError::UnknownGroup(group))?;
            g.children.iter().copied().collect()
        };
        for child in children {
            if let Some(task) = self.tasks.get_mut(&child) {
                if !task.is_terminal() {
                    task.cancel_flag = true;
                    if matches!(task.state, TaskState::Parked(_)) {
                        task.state = TaskState::Ready;
                        self.ready_queue.push_back(child);
                    }
                }
            }
        }
        Ok(())
    }

    /// Close the group to new spawns.
    ///
    /// Future `group_spawn` calls on this group will return `GroupClosed`.
    pub fn group_close(&mut self, group: GroupId) -> Result<(), ConcurrencyError> {
        self.get_group_mut(group)?.closed = true;
        Ok(())
    }

    // ── Channel operations ────────────────────────────────────────────────────

    /// Create a new bounded channel.  `capacity == 0` = rendezvous.
    pub fn chan_new(&mut self, capacity: usize) -> Result<ChannelId, ConcurrencyError> {
        let id = self.alloc_channel()?;
        self.channels.insert(id, ChannelEntry::new(capacity));
        Ok(id)
    }

    /// Send a value.  Parks the current task if the channel is full.
    pub fn chan_send(&mut self, ch: ChannelId, value: Value) -> Result<SendResult, ConcurrencyError> {
        let channel = self.get_channel_mut(ch)?;
        if channel.closed {
            return Err(ConcurrencyError::ChannelClosed(ch));
        }
        match channel.try_enqueue(value.clone()) {
            TryEnqueueResult::Enqueued => Ok(SendResult::Sent),
            TryEnqueueResult::DeliveredToWaiter(recv_h, _) => {
                // Wake the receiver.
                if let Some(t) = self.tasks.get_mut(&recv_h) {
                    t.wake();
                }
                self.ready_queue.push_back(recv_h);
                Ok(SendResult::Sent)
            }
            TryEnqueueResult::Full => {
                let current = self.require_current()?;
                let channel = self.get_channel_mut(ch)?;
                channel.send_waiters.push_back((current, value));
                self.park_current(ParkReason::ChanSend(ch))?;
                Ok(SendResult::Parked)
            }
            TryEnqueueResult::Closed => Err(ConcurrencyError::ChannelClosed(ch)),
        }
    }

    /// Receive a value.  Parks the current task if the channel is empty.
    pub fn chan_recv(&mut self, ch: ChannelId) -> Result<RecvResult, ConcurrencyError> {
        // call try_dequeue() immediately so the &mut borrow of channels ends before
        // we re-borrow self in the match arms below (NLL: borrow released on return).
        match self.get_channel_mut(ch)?.try_dequeue() {
            TryDequeueResult::Received(value, woken_sender) => {
                if let Some(sender) = woken_sender {
                    if let Some(t) = self.tasks.get_mut(&sender) {
                        t.wake();
                    }
                    self.ready_queue.push_back(sender);
                }
                Ok(RecvResult::Received(value))
            }
            TryDequeueResult::Empty => {
                let current = self.require_current()?;
                let channel = self.get_channel_mut(ch)?;
                channel.recv_waiters.push_back(current);
                self.park_current(ParkReason::ChanRecv(ch))?;
                Ok(RecvResult::Parked)
            }
            TryDequeueResult::Closed => Ok(RecvResult::Closed),
        }
    }

    /// Non-blocking send.  Returns `true` if the value was accepted.
    pub fn chan_try_send(&mut self, ch: ChannelId, value: Value) -> Result<bool, ConcurrencyError> {
        let channel = self.get_channel_mut(ch)?;
        if channel.closed {
            return Err(ConcurrencyError::ChannelClosed(ch));
        }
        match channel.try_enqueue(value) {
            TryEnqueueResult::Enqueued => Ok(true),
            TryEnqueueResult::DeliveredToWaiter(recv_h, _) => {
                if let Some(t) = self.tasks.get_mut(&recv_h) {
                    t.wake();
                }
                self.ready_queue.push_back(recv_h);
                Ok(true)
            }
            TryEnqueueResult::Full    => Ok(false),
            TryEnqueueResult::Closed  => Err(ConcurrencyError::ChannelClosed(ch)),
        }
    }

    /// Non-blocking receive.  Returns `Some(value)` if one was available.
    pub fn chan_try_recv(&mut self, ch: ChannelId) -> Result<Option<Value>, ConcurrencyError> {
        // Same NLL pattern as chan_recv: borrow ends when try_dequeue() returns.
        match self.get_channel_mut(ch)?.try_dequeue() {
            TryDequeueResult::Received(value, woken_sender) => {
                if let Some(sender) = woken_sender {
                    if let Some(t) = self.tasks.get_mut(&sender) {
                        t.wake();
                    }
                    self.ready_queue.push_back(sender);
                }
                Ok(Some(value))
            }
            TryDequeueResult::Empty  => Ok(None),
            TryDequeueResult::Closed => Ok(None),
        }
    }

    /// Close the send-side of a channel.  Wakes all parked receivers.
    pub fn chan_close(&mut self, ch: ChannelId) -> Result<(), ConcurrencyError> {
        let channel = self.channels.get_mut(&ch).ok_or(ConcurrencyError::UnknownChannel(ch))?;
        if channel.closed {
            return Err(ConcurrencyError::AlreadyClosed(ch));
        }
        channel.closed = true;
        // Collect recv waiters to wake.
        let waiters: Vec<TaskHandle> = channel.recv_waiters.drain(..).collect();
        // Also wake send waiters (they'll see ChannelClosed on next send attempt).
        let send_waiters: Vec<TaskHandle> = channel.send_waiters.iter().map(|(h, _)| *h).collect();
        channel.send_waiters.clear();
        for waiter in waiters.into_iter().chain(send_waiters) {
            if let Some(t) = self.tasks.get_mut(&waiter) {
                if t.wake() {
                    self.ready_queue.push_back(waiter);
                }
            }
        }
        Ok(())
    }

    // ── Select operations ─────────────────────────────────────────────────────

    /// Create a new empty select set.
    pub fn select_new(&mut self) -> Result<SelectSetId, ConcurrencyError> {
        let id = self.alloc_select()?;
        self.select_sets.insert(id, SelectSetEntry::new());
        Ok(id)
    }

    /// Register a recv arm.
    pub fn select_recv(&mut self, set: SelectSetId, ch: ChannelId) -> Result<ArmId, ConcurrencyError> {
        let _ = self.channels.get(&ch).ok_or(ConcurrencyError::UnknownChannel(ch))?;
        let entry = self.get_select_mut(set)?;
        Ok(entry.add_arm(SelectArm::Recv { ch }))
    }

    /// Register a send arm.
    pub fn select_send(&mut self, set: SelectSetId, ch: ChannelId, value: Value) -> Result<ArmId, ConcurrencyError> {
        let _ = self.channels.get(&ch).ok_or(ConcurrencyError::UnknownChannel(ch))?;
        let entry = self.get_select_mut(set)?;
        Ok(entry.add_arm(SelectArm::Send { ch, value }))
    }

    /// Register a task-join arm.
    pub fn select_join(&mut self, set: SelectSetId, task: TaskHandle) -> Result<ArmId, ConcurrencyError> {
        let _ = self.get_task(task)?;
        let entry = self.get_select_mut(set)?;
        Ok(entry.add_arm(SelectArm::Join { task }))
    }

    /// Register a timer arm.
    pub fn select_timer(&mut self, set: SelectSetId, deadline: Deadline) -> Result<ArmId, ConcurrencyError> {
        let entry = self.get_select_mut(set)?;
        Ok(entry.add_arm(SelectArm::Timer { deadline }))
    }

    /// Register a cancel-check arm.
    pub fn select_cancel(&mut self, set: SelectSetId, token: CancelTokenId) -> Result<ArmId, ConcurrencyError> {
        // Check if the token exists.
        if !self.tokens.contains_key(&token) {
            return Err(ConcurrencyError::UnknownCancelToken(token));
        }
        let arm_id = {
            let entry = self.get_select_mut(set)?;
            entry.add_arm(SelectArm::Cancel { token })
        };
        // Register this select set arm with the token.
        // Use get_mut with proper error return rather than unwrap, so a
        // concurrent-looking (but impossible in practice) removal doesn't panic.
        let token_entry = self.tokens.get_mut(&token)
            .ok_or(ConcurrencyError::UnknownCancelToken(token))?;
        token_entry.select_waiters.push((set, arm_id));
        Ok(arm_id)
    }

    /// Add a default (no-wait) arm.
    pub fn select_default(&mut self, set: SelectSetId) -> Result<ArmId, ConcurrencyError> {
        let entry = self.get_select_mut(set)?;
        Ok(entry.add_arm(SelectArm::Default))
    }

    /// Poll all arms.  If any arm is immediately ready, return its `SelectResult`.
    /// If a default arm exists and nothing is ready, fire the default.
    /// Otherwise park the current task.
    pub fn select_wait(&mut self, set: SelectSetId) -> Result<Option<SelectResult>, ConcurrencyError> {
        // Snapshot of arms for polling (we can't borrow self while iterating arms
        // and also calling channel/task methods).
        let arm_snapshot: Vec<(ArmId, SelectArm)> = {
            let entry = self.get_select_mut(set)?;
            entry.arms.iter().enumerate()
                .map(|(i, arm)| (ArmId(i as u32), arm.clone()))
                .collect()
        };

        // Poll each arm in order (deterministic: lowest ArmId wins on tie).
        for (arm_id, arm) in &arm_snapshot {
            if let Some(result) = self.poll_arm(set, *arm_id, arm)? {
                return Ok(Some(result));
            }
        }

        // No arm was ready.  Fire default if present.
        // Use get_select_mut (returns Err) rather than [] (panics) for robustness.
        let has_default = self.get_select_mut(set)?.has_default;
        if has_default {
            let default_id = arm_snapshot.iter()
                .find(|(_, a)| matches!(a, SelectArm::Default))
                .map(|(id, _)| *id);
            if let Some(id) = default_id {
                return Ok(Some(SelectResult {
                    arm_id: id,
                    kind: SelectArmKind::Default,
                    value: None,
                    status: SelectStatus::Default,
                }));
            }
        }

        // Park the current task.
        let current = self.require_current()?;
        self.get_select_mut(set)?.waiter = Some(current);
        self.park_current(ParkReason::Select(set))?;
        Ok(None)
    }

    /// Poll a single select arm.  Returns `Some(SelectResult)` if the arm fires.
    fn poll_arm(&mut self, _set: SelectSetId, arm_id: ArmId, arm: &SelectArm)
        -> Result<Option<SelectResult>, ConcurrencyError>
    {
        match arm {
            SelectArm::Recv { ch } => {
                let ch = *ch;
                let channel = match self.channels.get_mut(&ch) {
                    Some(c) => c,
                    None => return Err(ConcurrencyError::UnknownChannel(ch)),
                };
                match channel.try_dequeue() {
                    TryDequeueResult::Received(value, woken_sender) => {
                        if let Some(sender) = woken_sender {
                            self.enqueue_ready(sender);
                        }
                        Ok(Some(SelectResult {
                            arm_id,
                            kind: SelectArmKind::Recv,
                            value: Some(value),
                            status: SelectStatus::Ready,
                        }))
                    }
                    TryDequeueResult::Closed => Ok(Some(SelectResult {
                        arm_id,
                        kind: SelectArmKind::Recv,
                        value: None,
                        status: SelectStatus::Closed,
                    })),
                    TryDequeueResult::Empty => Ok(None),
                }
            }
            SelectArm::Send { ch, value } => {
                let ch = *ch;
                let value = value.clone();
                let channel = match self.channels.get_mut(&ch) {
                    Some(c) => c,
                    None => return Err(ConcurrencyError::UnknownChannel(ch)),
                };
                if channel.closed {
                    return Err(ConcurrencyError::ChannelClosed(ch));
                }
                match channel.try_enqueue(value) {
                    TryEnqueueResult::Enqueued => Ok(Some(SelectResult {
                        arm_id,
                        kind: SelectArmKind::Send,
                        value: None,
                        status: SelectStatus::Ready,
                    })),
                    TryEnqueueResult::DeliveredToWaiter(recv_h, _) => {
                        self.enqueue_ready(recv_h);
                        Ok(Some(SelectResult {
                            arm_id,
                            kind: SelectArmKind::Send,
                            value: None,
                            status: SelectStatus::Ready,
                        }))
                    }
                    TryEnqueueResult::Full | TryEnqueueResult::Closed => Ok(None),
                }
            }
            SelectArm::Join { task } => {
                let task = *task;
                let t = match self.tasks.get(&task) {
                    Some(t) => t,
                    None => return Err(ConcurrencyError::UnknownTask(task)),
                };
                if t.is_done() {
                    let value = t.return_value.clone();
                    Ok(Some(SelectResult {
                        arm_id,
                        kind: SelectArmKind::Join,
                        value,
                        status: SelectStatus::Ready,
                    }))
                } else {
                    Ok(None)
                }
            }
            SelectArm::Timer { deadline } => {
                if self.clock >= *deadline {
                    Ok(Some(SelectResult {
                        arm_id,
                        kind: SelectArmKind::Timer,
                        value: None,
                        status: SelectStatus::TimedOut,
                    }))
                } else {
                    Ok(None)
                }
            }
            SelectArm::Cancel { token } => {
                let token = *token;
                let t = self.tokens.get(&token).ok_or(ConcurrencyError::UnknownCancelToken(token))?;
                if t.cancelled {
                    Ok(Some(SelectResult {
                        arm_id,
                        kind: SelectArmKind::Cancel,
                        value: None,
                        status: SelectStatus::Cancelled,
                    }))
                } else {
                    Ok(None)
                }
            }
            SelectArm::Default => Ok(None), // handled by select_wait after the loop
        }
    }

    // ── Cancel token ──────────────────────────────────────────────────────────

    /// Allocate a new cancel token.
    pub fn new_cancel_token(&mut self) -> Result<CancelTokenId, ConcurrencyError> {
        let id = self.alloc_token()?;
        self.tokens.insert(id, CancelTokenEntry::new());
        Ok(id)
    }

    // ── Scheduler run-loop ────────────────────────────────────────────────────

    /// Pick the next runnable task from the ready queue.
    ///
    /// Returns `None` if the ready queue is empty.  The caller should then
    /// either advance the clock (deterministic mode) or block on OS events.
    pub fn pick_next(&mut self) -> Option<TaskHandle> {
        while let Some(handle) = self.ready_queue.pop_front() {
            // Skip tasks that are no longer in the Ready state (they may have
            // been cancelled or completed while queued).
            if let Some(task) = self.tasks.get(&handle) {
                if task.state == TaskState::Ready {
                    return Some(handle);
                }
            }
        }
        None
    }

    /// Set the current executing task and mark it `Running`.
    pub fn set_current(&mut self, task: TaskHandle) {
        if let Some(entry) = self.tasks.get_mut(&task) {
            entry.state = TaskState::Running;
        }
        self.current = Some(task);
    }

    /// Complete the current task normally.  Wakes any join waiters.
    pub fn complete_current(&mut self, value: Value) -> Result<(), ConcurrencyError> {
        let current = self.require_current()?;
        // Collect join waiters before mutating task state.
        let join_waiters: Vec<TaskHandle> = {
            let task = self.get_task_mut(current)?;
            task.state = TaskState::Completed;
            task.return_value = Some(value.clone());
            task.join_waiters.drain(..).collect()
        };
        self.current = None;
        // Wake all join waiters.
        for waiter in &join_waiters {
            self.enqueue_ready(*waiter);
        }
        // Notify parent group.
        let parent_group = self.tasks[&current].parent_group;
        if let Some(group_id) = parent_group {
            self.on_child_complete(group_id, current, Some(value), None);
        }
        Ok(())
    }

    /// Mark the current task as failed.  Wakes any join waiters.
    pub fn fail_current(&mut self, error: String) -> Result<(), ConcurrencyError> {
        let current = self.require_current()?;
        let join_waiters: Vec<TaskHandle> = {
            let task = self.get_task_mut(current)?;
            task.state = TaskState::Failed;
            task.error = Some(error.clone());
            task.join_waiters.drain(..).collect()
        };
        self.current = None;
        for waiter in &join_waiters {
            self.enqueue_ready(*waiter);
        }
        let parent_group = self.tasks[&current].parent_group;
        if let Some(group_id) = parent_group {
            self.on_child_complete(group_id, current, None, Some(error));
        }
        Ok(())
    }

    /// Internal: notify a group that one of its children reached a terminal state.
    fn on_child_complete(&mut self, group_id: GroupId, child: TaskHandle, value: Option<Value>, error: Option<String>) {
        let (resolved, join_waiters, policy) = {
            let group = match self.groups.get_mut(&group_id) {
                Some(g) => g,
                None => return,
            };
            let resolved = group.remove_child(child, value, error.clone());
            let waiters: Vec<TaskHandle> = if resolved {
                group.join_waiters.drain(..).collect()
            } else {
                vec![]
            };
            (resolved, waiters, group.policy)
        };

        // Under FailFast: if there was an error, cancel siblings.
        if error.is_some() && policy == GroupPolicy::FailFast {
            let _ = self.group_cancel(group_id);
        }

        if resolved {
            for waiter in join_waiters {
                self.enqueue_ready(waiter);
            }
        }
    }

    /// Advance the virtual clock to `now` and wake any sleeping tasks.
    ///
    /// Only meaningful in deterministic mode.  In production mode this is a
    /// no-op (the real clock is used instead).
    pub fn advance_clock(&mut self, now: Deadline) {
        if !self.deterministic { return; }
        self.clock = now;
        // Wake all sleeping tasks whose deadline has passed.
        while let Some(Reverse((deadline, _))) = self.timers.peek() {
            if *deadline > now { break; }
            let Reverse((_, handle)) = self.timers.pop().unwrap();
            if let Some(task) = self.tasks.get_mut(&handle) {
                if matches!(task.state, TaskState::Parked(ParkReason::Sleep(_))) {
                    task.state = TaskState::Ready;
                    self.ready_queue.push_back(handle);
                }
            }
        }
    }

    /// Returns the current virtual clock value.
    ///
    /// In deterministic mode this starts at `Deadline::ZERO` and advances only
    /// via `advance_clock`.  In production mode this always returns
    /// `Deadline::ZERO` (wall-clock integration is out of scope for v0.1).
    pub fn current_time(&self) -> Deadline {
        self.clock
    }

    /// Returns the seed that was passed to `Scheduler::deterministic`.
    ///
    /// Always returns `0` for schedulers created with `Scheduler::new`.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Returns `true` if all spawned tasks have reached a terminal state.
    pub fn is_done(&self) -> bool {
        self.tasks.values().all(|t| t.is_terminal() || t.state == TaskState::Ready)
            && self.ready_queue.is_empty()
    }

    /// Returns `true` if the scheduler has at least one ready task.
    pub fn has_ready(&self) -> bool {
        !self.ready_queue.is_empty()
    }

    /// Returns the state of a task (for inspection / testing).
    pub fn task_state(&self, h: TaskHandle) -> Option<TaskState> {
        self.tasks.get(&h).map(|t| t.state)
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
