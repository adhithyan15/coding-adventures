# LANG28B — `vm-concurrency` Single-Thread Cooperative Scheduler

## Context

LANG28A (interpreter-ir v0.3.0) added 27 concurrency opcodes to the IIR
taxonomy: task, group, channel, and select families.  LANG28B delivers the
**first runnable implementation**: a single-thread cooperative scheduler crate
(`vm-concurrency`) that `vm-core` can call into whenever it executes one of
those opcodes.

The full LANG28 vision (M:N workers, debugger integration, native event
backends, JIT/AOT paths, host-VM adapters) is described in
`LANG28-vm-concurrency.md`.  This document scopes LANG28B to the
**interpreter-only, single-thread subset** that makes the semantics testable
before OS parallelism or JIT paths enter the picture.

---

## Deliverable

One new Rust crate: `code/packages/rust/vm-concurrency/`

```
vm-concurrency/
  Cargo.toml
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs          — re-exports + crate doc
    task.rs         — TaskId, TaskState, TaskHandle, TaskEntry
    channel.rs      — ChannelId, ChannelEntry, send/recv queues
    group.rs        — GroupId, GroupEntry, failure policies
    select.rs       — SelectSetId, SelectArm, SelectResult
    timer.rs        — Deadline, TimerHeap
    cancel.rs       — CancelToken, CancelTokenId
    scheduler.rs    — Scheduler — the central coordinator
    error.rs        — ConcurrencyError
    deterministic.rs — DeterministicClock, DeterministicScheduler wrapper
  tests/
    test_task.rs    — task spawn/yield/join/cancel
    test_channel.rs — chan_new/send/recv/try_send/try_recv/close
    test_group.rs   — group_new/spawn/join/cancel/close
    test_select.rs  — select_new/.../wait/default
    test_cancel.rs  — cancellation propagation
    test_deterministic.rs — deterministic mode, virtual clock
```

---

## Architecture rationale

### Why a separate crate, not adding to vm-core?

`vm-core` is the generic register interpreter.  It dispatches IIR opcodes one
by one; it knows nothing about tasks.  Mixing scheduler state into `vm-core`
would make `vm-core` bulky and would force every `vm-core` user (JIT, AOT,
host-VM adapters) to carry scheduler machinery even if they implement
concurrency differently.

The clean design:

```
vm-core     — pure single-instruction interpreter
    calls into vm-concurrency for concurrency opcodes
    |
    v
vm-concurrency — scheduler, task table, channels, groups, select, timers
```

`vm-core` gets a new `OpcodeHandler` (or a small hook via a `Scheduler` trait
object) that routes concurrency opcodes to `vm-concurrency`.  The rest of
`vm-core` is untouched.

---

## Public API (Rust)

### `Scheduler`

The central stateful object.  One per program execution.

```rust
pub struct Scheduler { /* private */ }

impl Scheduler {
    /// Create a new scheduler with production (non-deterministic) settings.
    pub fn new() -> Self;

    /// Create a scheduler in deterministic mode with a fixed seed.
    ///
    /// In deterministic mode:
    /// - ready-queue ordering is FIFO by task-spawn order (no work-stealing);
    /// - virtual clock is used — time never advances unless `advance_clock` is
    ///   called explicitly;
    /// - select tie-breaking uses the seed.
    pub fn deterministic(seed: u64) -> Self;

    /// Spawn a new task.
    ///
    /// The task starts in `Ready` state.  It is the caller's responsibility
    /// to push the first interpreter frame before the task can actually run.
    pub fn task_spawn(&mut self) -> TaskHandle;

    /// Get the handle for the "current" task (the one currently executing).
    ///
    /// Returns `None` if the scheduler has no current task (e.g. the main
    /// program context before any task is selected).
    pub fn task_current(&self) -> Option<TaskHandle>;

    /// Mark the current task as yielded.  It moves from `Running → Ready`.
    ///
    /// Returns `Err(ConcurrencyError::NoCurrent)` if no task is running.
    pub fn task_yield(&mut self) -> Result<(), ConcurrencyError>;

    /// Park the current task until `deadline`.
    ///
    /// Returns `Err(ConcurrencyError::NoCurrent)` if no task is running.
    pub fn task_sleep(&mut self, deadline: Deadline) -> Result<(), ConcurrencyError>;

    /// Register a join waiter.
    ///
    /// If `target` is already completed, returns `Some(value)` immediately
    /// without parking.  Otherwise parks the current task and returns `None`.
    pub fn task_join(&mut self, target: TaskHandle) -> Result<Option<Value>, ConcurrencyError>;

    /// Request cooperative cancellation of `target`.
    ///
    /// Sets the cancel flag on the target.  The target will observe the flag
    /// at its next `is_parking` safepoint.
    pub fn task_cancel(&mut self, target: TaskHandle, token: CancelTokenId)
        -> Result<bool, ConcurrencyError>;

    /// Check whether the current task's cancel flag is set and clear it.
    ///
    /// Returns `true` if the task was cancelled.
    pub fn task_check_cancel(&mut self) -> Result<bool, ConcurrencyError>;

    /// Detach `target` from its parent group (if any).
    pub fn task_detach(&mut self, target: TaskHandle) -> Result<(), ConcurrencyError>;

    // ── Task groups ──────────────────────────────────────────────────────────

    pub fn group_new(&mut self, policy: GroupPolicy) -> GroupId;
    pub fn group_spawn(&mut self, group: GroupId) -> Result<TaskHandle, ConcurrencyError>;
    pub fn group_join(&mut self, group: GroupId) -> Result<Option<Vec<Value>>, ConcurrencyError>;
    pub fn group_cancel(&mut self, group: GroupId) -> Result<(), ConcurrencyError>;
    pub fn group_close(&mut self, group: GroupId) -> Result<(), ConcurrencyError>;

    // ── Channels ─────────────────────────────────────────────────────────────

    pub fn chan_new(&mut self, capacity: usize) -> ChannelId;
    pub fn chan_send(&mut self, ch: ChannelId, value: Value) -> Result<SendResult, ConcurrencyError>;
    pub fn chan_recv(&mut self, ch: ChannelId) -> Result<RecvResult, ConcurrencyError>;
    pub fn chan_try_send(&mut self, ch: ChannelId, value: Value) -> Result<bool, ConcurrencyError>;
    pub fn chan_try_recv(&mut self, ch: ChannelId) -> Result<Option<Value>, ConcurrencyError>;
    pub fn chan_close(&mut self, ch: ChannelId) -> Result<(), ConcurrencyError>;

    // ── Select ───────────────────────────────────────────────────────────────

    pub fn select_new(&mut self) -> SelectSetId;
    pub fn select_recv(&mut self, set: SelectSetId, ch: ChannelId) -> Result<ArmId, ConcurrencyError>;
    pub fn select_send(&mut self, set: SelectSetId, ch: ChannelId, value: Value)
        -> Result<ArmId, ConcurrencyError>;
    pub fn select_join(&mut self, set: SelectSetId, task: TaskHandle)
        -> Result<ArmId, ConcurrencyError>;
    pub fn select_timer(&mut self, set: SelectSetId, deadline: Deadline)
        -> Result<ArmId, ConcurrencyError>;
    pub fn select_cancel(&mut self, set: SelectSetId, token: CancelTokenId)
        -> Result<ArmId, ConcurrencyError>;
    /// Poll all arms.  If any arm is immediately ready, return its result.
    /// Otherwise park the current task.  Returns `None` if parked.
    pub fn select_wait(&mut self, set: SelectSetId) -> Result<Option<SelectResult>, ConcurrencyError>;
    /// Add a default (no-wait) arm to the select set.
    pub fn select_default(&mut self, set: SelectSetId) -> Result<ArmId, ConcurrencyError>;

    // ── Scheduler run-loop ───────────────────────────────────────────────────

    /// Pick the next ready task to run.
    ///
    /// Returns `None` if the scheduler has no ready task (all parked or done).
    /// In that case the caller should advance the clock / wake timers first.
    pub fn pick_next(&mut self) -> Option<TaskHandle>;

    /// Set the current task (the one whose opcodes are executing).
    pub fn set_current(&mut self, task: TaskHandle);

    /// Complete the current task with a return value.  Wakes any join waiters.
    pub fn complete_current(&mut self, value: Value) -> Result<(), ConcurrencyError>;

    /// Mark the current task as failed (runtime error or trap).  Wakes join waiters.
    pub fn fail_current(&mut self, error: String) -> Result<(), ConcurrencyError>;

    /// Advance the virtual clock to `now` and wake any sleeping tasks whose
    /// deadline has passed.  Only has an effect in deterministic mode.
    pub fn advance_clock(&mut self, now: Deadline);

    /// Returns `true` if all tasks are completed or failed.
    pub fn is_done(&self) -> bool;

    /// Returns `true` if the scheduler has at least one ready task.
    pub fn has_ready(&self) -> bool;

    /// Cancel token factory.
    pub fn new_cancel_token(&mut self) -> CancelTokenId;
}
```

### Key types

```rust
/// Opaque handle to a scheduled task.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TaskHandle(u32);

/// Stable identifier for a task group.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GroupId(u32);

/// Stable identifier for a channel.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChannelId(u32);

/// Stable identifier for a select set.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SelectSetId(u32);

/// Index of one arm within a select set.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ArmId(u32);

/// Opaque cancel token.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CancelTokenId(u32);

/// Monotonic deadline: nanoseconds since an arbitrary epoch.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Deadline(u64);

impl Deadline {
    pub const ZERO: Deadline = Deadline(0);
    pub fn from_nanos(ns: u64) -> Self;
    pub fn as_nanos(self) -> u64;
    pub fn plus_millis(self, ms: u64) -> Self;
}

/// A VM value that can move across task/channel boundaries.
///
/// Re-uses the same `Value` enum as `vm-core`, or an isomorphic copy.
/// (Exact type TBD when vm-core is updated.)
pub type Value = vm_core::value::Value;

/// Result of a blocking send attempt.
pub enum SendResult {
    /// Value was delivered immediately.
    Sent,
    /// Channel was full; current task parked.  Will unpark when space is free.
    Parked,
}

/// Result of a blocking recv attempt.
pub enum RecvResult {
    /// Value received immediately.
    Received(Value),
    /// Channel was empty; current task parked.  Will unpark when a value arrives.
    Parked,
    /// Channel was closed and empty.
    Closed,
}

/// Result of a completed select wait.
pub struct SelectResult {
    pub arm_id: ArmId,
    pub kind: SelectArmKind,
    pub value: Option<Value>,
    pub status: SelectStatus,
}

pub enum SelectArmKind { Recv, Send, Join, Timer, Cancel, Default }
pub enum SelectStatus { Ready, Closed, Cancelled, TimedOut }

/// Task lifecycle state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskState {
    New,
    Ready,
    Running,
    Parked(ParkReason),
    CancelRequested,
    Completed,
    Failed,
    Cancelled,
    Detached,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParkReason {
    Yield,
    Sleep,
    Join(TaskHandle),
    ChanSend(ChannelId),
    ChanRecv(ChannelId),
    GroupJoin(GroupId),
    Select(SelectSetId),
}

/// Group failure policy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GroupPolicy {
    /// First child failure cancels all siblings and propagates.
    FailFast,
    /// All children run to completion; errors are collected.
    CollectErrors,
    /// Errors are ignored; children run to completion.
    Supervise,
}
```

### `ConcurrencyError`

```rust
#[derive(Clone, PartialEq, Debug)]
pub enum ConcurrencyError {
    /// No current task is executing.
    NoCurrent,
    /// TaskHandle refers to a task that does not exist.
    UnknownTask(TaskHandle),
    /// GroupId refers to a group that does not exist.
    UnknownGroup(GroupId),
    /// ChannelId refers to a channel that does not exist.
    UnknownChannel(ChannelId),
    /// SelectSetId refers to a set that does not exist.
    UnknownSelectSet(SelectSetId),
    /// CancelTokenId refers to a token that does not exist.
    UnknownCancelToken(CancelTokenId),
    /// Sending to a closed channel.
    ChannelClosed(ChannelId),
    /// Closing a channel that is already closed.
    AlreadyClosed(ChannelId),
    /// Spawning into a closed group.
    GroupClosed(GroupId),
    /// Too many tasks, channels, or groups (u32 ID space exhausted).
    ResourceExhausted,
}

impl std::fmt::Display for ConcurrencyError { ... }
impl std::error::Error for ConcurrencyError {}
```

---

## Internal data structures

### Task table

```
HashMap<TaskHandle, TaskEntry>
  TaskEntry {
      state: TaskState,
      cancel_flag: bool,
      parent_group: Option<GroupId>,
      return_value: Option<Value>,   // set on Complete
      error: Option<String>,          // set on Failed
      join_waiters: Vec<TaskHandle>,  // tasks waiting to join this one
  }
```

### Ready queue

Single-ended `VecDeque<TaskHandle>` in production mode (FIFO).

In deterministic mode: `VecDeque<TaskHandle>` sorted by `TaskHandle` id (stable
insertion order from spawn sequence).

### Channel table

```
HashMap<ChannelId, ChannelEntry>
  ChannelEntry {
      capacity: usize,          // 0 = rendezvous
      buffer: VecDeque<Value>,
      closed: bool,
      send_waiters: VecDeque<(TaskHandle, Value)>,
      recv_waiters: VecDeque<TaskHandle>,
  }
```

### Group table

```
HashMap<GroupId, GroupEntry>
  GroupEntry {
      policy: GroupPolicy,
      closed: bool,
      children: HashSet<TaskHandle>,
      errors: Vec<(TaskHandle, String)>,
      join_waiters: Vec<TaskHandle>,  // tasks waiting for group_join
  }
```

### Timer heap

Min-heap ordered by `Deadline`.

```
BinaryHeap<Reverse<(Deadline, TaskHandle)>>
```

In deterministic mode the clock only advances via `advance_clock()`; the heap
never fires spontaneously.

### Select set table

```
HashMap<SelectSetId, SelectSetEntry>
  SelectSetEntry {
      arms: Vec<SelectArm>,
      has_default: bool,
      waiter: Option<TaskHandle>,
  }

enum SelectArm {
    Recv { ch: ChannelId },
    Send { ch: ChannelId, value: Value },
    Join { task: TaskHandle },
    Timer { deadline: Deadline },
    Cancel { token: CancelTokenId },
    Default,
}
```

---

## Scheduler run-loop contract (for vm-core)

When `vm-core` calls a concurrency opcode on a `Scheduler`, the scheduler
may return one of two outcomes:

1. **`Completed`** — the opcode finished synchronously (e.g. `chan_try_recv` on
   a non-empty channel).  `vm-core` continues executing the current task.

2. **`Parked`** — the current task was parked.  `vm-core` must call
   `scheduler.pick_next()` to get the next runnable task, then switch to
   executing that task's frame.

`vm-core` runs the following outer loop:

```
loop {
    let Some(task) = scheduler.pick_next() else {
        scheduler.advance_clock(...);          // deterministic: caller drives
        if scheduler.is_done() { break; }
        continue;
    };
    scheduler.set_current(task);
    loop {
        let instr = fetch_next_instr(task);
        let done = dispatch(instr, &mut scheduler);
        if done == TaskParked { break; }       // switch to pick_next
        if done == TaskComplete { break; }
        if done == TaskFailed { break; }
    }
}
```

This outer loop lives in a new `vm-core` function
`VMCore::run_concurrent(scheduler: &mut Scheduler)`.

---

## Integration with vm-core

`vm-core` dispatch.rs handles concurrency opcodes by calling into `Scheduler`.

Changes needed in `vm-core`:
1. Add optional `scheduler: Option<Box<dyn ConcurrencyScheduler>>` to `VMCore`.
2. In `dispatch.rs`, match on `is_concurrency(instr.op)` and route to scheduler.
3. `ConcurrencyScheduler` is a trait that `Scheduler` implements — this allows
   test doubles and future M:N scheduler swapping.

The `ConcurrencyScheduler` trait mirrors `Scheduler`'s public API as `dyn`
methods.

`vm-core` does NOT take a direct dependency on `vm-concurrency` at the type
level if the trait-object approach is used, keeping `vm-core` free of the
heavier scheduler machinery in AOT/JIT paths that handle concurrency
differently.

---

## Dependencies

```toml
[dependencies]
interpreter-ir = { path = "../interpreter-ir" }   # ≥0.3.0 for concurrency opcodes
vm-core        = { path = "../vm-core" }            # for Value type

[dev-dependencies]
interpreter-ir = { path = "../interpreter-ir" }
vm-core        = { path = "../vm-core" }
```

---

## Test plan

### `test_task.rs` (≥15 tests)

| Test | What it checks |
|------|----------------|
| `task_spawn_assigns_new_handle` | spawn returns increasing IDs |
| `task_spawn_state_is_new` | state == New before first `pick_next` |
| `task_yield_moves_to_ready` | yield puts task back on ready queue |
| `task_join_immediate_complete` | join on already-complete task returns value |
| `task_join_parks_waiter` | join on running task parks the joiner |
| `task_join_wake_on_complete` | completing a task wakes its joiner |
| `task_cancel_sets_flag` | cancel sets flag; check_cancel reads it |
| `task_cancel_unknown_handle` | returns `UnknownTask` error |
| `task_detach_removes_from_group` | detached task no longer counted in group |
| `task_check_cancel_clears_flag` | second check_cancel returns false |
| `multi_task_round_robin` | three tasks yield in turn; order is FIFO |
| `task_sleep_parks_until_deadline` | sleeping task absent from ready queue |
| `task_sleep_wakes_on_advance_clock` | advance_clock past deadline wakes task |
| `task_fail_wakes_joiner` | failed task unparks joiner; value contains error |
| `task_complete_group_cleanup` | completing last group child runs group_join waiter |

### `test_channel.rs` (≥12 tests)

| Test | What it checks |
|------|----------------|
| `chan_new_bounded` | buffer capped at capacity |
| `chan_try_send_returns_false_when_full` | try_send on full → false |
| `chan_try_recv_returns_none_when_empty` | try_recv on empty → None |
| `chan_send_immediate_on_space` | send to non-full → Sent |
| `chan_recv_immediate_on_item` | recv from non-empty → Received |
| `chan_send_parks_when_full` | send to full channel parks the sender |
| `chan_recv_parks_when_empty` | recv from empty channel parks the receiver |
| `chan_send_wakes_blocked_recv` | sending unparks a waiting receiver |
| `chan_recv_wakes_blocked_send` | receiving unparks a waiting sender |
| `chan_close_wakes_recv_waiters` | closing channel delivers Closed to waiters |
| `chan_send_to_closed` | send to closed → ChannelClosed error |
| `chan_rendezvous` | capacity 0 — sender parks until receiver arrives |

### `test_group.rs` (≥8 tests)

| Test | What it checks |
|------|----------------|
| `group_new_empty` | group starts with no children |
| `group_spawn_adds_child` | spawn adds task to group |
| `group_join_parks_until_all_done` | join parks; unparks when all children complete |
| `group_join_fail_fast` | first failure cancels siblings |
| `group_cancel_marks_children` | all children get cancel flag |
| `group_close_rejects_spawn` | spawn on closed group returns GroupClosed |
| `group_collect_errors_completes` | failed children collected; join still resolves |
| `group_detached_child_not_counted` | detached child removed from group membership |

### `test_select.rs` (≥10 tests)

| Test | What it checks |
|------|----------------|
| `select_recv_fires_immediately` | recv arm fires when channel has data |
| `select_send_fires_immediately` | send arm fires when channel has space |
| `select_default_fires_when_nothing_ready` | default arm chosen |
| `select_wait_parks_when_no_arm_ready` | parks current task |
| `select_woken_by_chan_send` | another task sending wakes select receiver |
| `select_join_fires_on_task_complete` | task completion fires select join arm |
| `select_timer_fires_on_advance_clock` | timer arm fires after clock advance |
| `select_cancel_fires_on_cancel` | cancel token fires cancel arm |
| `select_fairness_deterministic` | deterministic mode picks lowest arm_id |
| `select_second_ready_arm_not_fired` | only one arm fires per wait |

### `test_cancel.rs` (≥5 tests)

| Test | What it checks |
|------|----------------|
| `cancel_token_created_uncancelled` | new token is not cancelled |
| `task_cancel_propagates_via_token` | cancel via token sets flag on task |
| `check_cancel_after_yield` | yield is a safepoint for cancel observation |
| `cancel_unblocks_select_arm` | cancelled token fires select_cancel arm |
| `cancel_unknown_token` | UnknownCancelToken error |

### `test_deterministic.rs` (≥5 tests)

| Test | What it checks |
|------|----------------|
| `deterministic_mode_fifo_order` | tasks run in spawn order |
| `deterministic_clock_starts_at_zero` | clock starts at Deadline::ZERO |
| `deterministic_clock_does_not_advance_spontaneously` | sleep task stays parked |
| `deterministic_advance_clock_wakes_timers` | clock advance fires timers in deadline order |
| `deterministic_seed_affects_select_tiebreak` | two seeds → different select arms |

Total: ≥55 tests.

---

## Non-goals for LANG28B

- M:N worker pools and OS threads (LANG28E)
- GC root enumeration across task stacks (LANG28C)
- Debugger task-list API (LANG28C)
- Native event backends, async I/O (LANG28D)
- JIT/AOT stack maps for parking points (LANG28G)
- Host VM backends: JVM, CLR, BEAM, WASM (LANG28G)
- `liblang-std` OS thread/process APIs (LANG28F)

Backends that encounter concurrency opcodes in this release should continue
returning `UnsupportedOp` from their validator.

---

## Definition of done for LANG28B

- `cargo test -p vm-concurrency` — ≥55 tests, ≥85% line coverage
- `cargo build --workspace` — clean
- `vm-core` can call into `vm-concurrency` via the `ConcurrencyScheduler` trait
  (proof of integration: one integration test that runs a simple two-task
  producer/consumer through `VMCore::run_concurrent`)
- Deterministic mode is exercised by all `test_deterministic.rs` tests
- Security review passes
- CHANGELOG.md and README.md present

---

## Files to create

| File | Action |
|------|--------|
| `code/specs/LANG28B-vm-concurrency-scheduler.md` | ✅ this file |
| `code/packages/rust/vm-concurrency/Cargo.toml` | CREATE |
| `code/packages/rust/vm-concurrency/BUILD` | CREATE (shell: `cargo test -p vm-concurrency`) |
| `code/packages/rust/vm-concurrency/README.md` | CREATE |
| `code/packages/rust/vm-concurrency/CHANGELOG.md` | CREATE |
| `code/packages/rust/vm-concurrency/src/{lib,task,channel,group,select,timer,cancel,scheduler,error,deterministic}.rs` | CREATE |
| `code/packages/rust/vm-concurrency/tests/{test_task,test_channel,test_group,test_select,test_cancel,test_deterministic}.rs` | CREATE |
| `code/packages/rust/Cargo.toml` | UPDATE (add `vm-concurrency` workspace member) |
| `code/packages/rust/vm-core/src/dispatch.rs` | UPDATE (route concurrency opcodes to `ConcurrencyScheduler` trait) |

---

## Version

`vm-concurrency` starts at `v0.1.0`.

`interpreter-ir` dependency must be `≥0.3.0` (for `is_concurrency`, `is_parking`,
and the 27 new opcode names added in LANG28A).
