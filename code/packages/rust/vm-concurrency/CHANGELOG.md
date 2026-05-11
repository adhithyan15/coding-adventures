# Changelog — vm-concurrency

All notable changes to this package follow
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) conventions.
Version numbers follow [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-05-11

### Added (LANG28B — cooperative scheduler)

#### Core types (`src/types.rs`)
- `TaskHandle(u32)` — opaque, `Copy`/`Hash`, stable task identifier.
- `GroupId(u32)`, `ChannelId(u32)`, `SelectSetId(u32)`, `ArmId(u32)`,
  `CancelTokenId(u32)` — same shape, one per resource family.
- `Deadline` — monotonic nanosecond timestamp with `from_nanos`, `as_nanos`,
  `plus_millis`, `ZERO`, `Ord`/`PartialOrd`.
- `TaskState` — `New | Ready | Running | Parked(ParkReason) | CancelRequested |
  Completed | Failed | Cancelled | Detached`.
- `ParkReason` — `Yield | Sleep(Deadline) | Join(TaskHandle) |
  ChanSend(ChannelId) | ChanRecv(ChannelId) | GroupJoin(GroupId) |
  Select(SelectSetId)`.
- `GroupPolicy` — `FailFast` (default), `CollectErrors`, `Supervise`.
- `SendResult` — `Sent | Parked`.
- `RecvResult` — `Received(Value) | Parked | Closed`.
- `SelectResult` struct: `arm_id`, `kind`, `value`, `status`.
- `SelectArmKind` — `Recv | Send | Join | Timer | Cancel | Default`.
- `SelectStatus` — `Ready | Closed | Cancelled | TimedOut | Default`.
- Re-export of `vm_core::value::Value`.

#### Error type (`src/error.rs`)
- `ConcurrencyError` enum: `NoCurrent`, `UnknownTask`, `UnknownGroup`,
  `UnknownChannel`, `UnknownSelectSet`, `UnknownCancelToken`,
  `ChannelClosed`, `AlreadyClosed`, `GroupClosed`, `ResourceExhausted`.
- Implements `Display`, `std::error::Error`, `Clone`, `PartialEq`, `Debug`.

#### Task entry (`src/task.rs`)
- `TaskEntry` struct: `state`, `cancel_flag`, `detached`, `name`,
  `parent_group`, `join_waiters`, `return_value`, `error`.
- Methods: `new()`, `is_terminal()`, `is_done()`, `park(reason)`, `wake()`.

#### Channel entry (`src/channel.rs`)
- `ChannelEntry` with FIFO buffer, blocked send/recv waiter queues, close flag.
- `TryEnqueueResult` — `Enqueued | DeliveredToWaiter | Full | Closed`.
- `TryDequeueResult` — `Received(Value, Option<waiter>) | Empty | Closed`.
- Rendezvous mode: capacity 0 delivers directly to waiting receiver.

#### Group entry (`src/group.rs`)
- `GroupEntry`: policy, closed flag, child set, error/value lists, join waiters.
- FailFast policy: first failing child cancels all siblings.
- CollectErrors: all errors collected; group resolves only when all children
  reach a terminal state.

#### Select set entry (`src/select.rs`)
- `SelectArm` enum: `Recv | Send | Join | Timer | Cancel | Default`.
- `SelectSetEntry`: arm list, `has_default` flag, optional parked waiter,
  resolved result.
- `add_arm(arm)` → `ArmId` (sequential from 0).

#### Cancel token (`src/cancel.rs`)
- `CancelTokenEntry`: `cancelled` flag, `select_waiters` list.
- `cancel()` → returns drained `Vec<(SelectSetId, ArmId)>` of wake targets.

#### Scheduler (`src/scheduler.rs`)
Complete `Scheduler` struct with all 27 opcode methods + run-loop API:

**Task operations:**
`task_spawn`, `task_current`, `task_yield`, `task_sleep`, `task_join`,
`task_cancel`, `task_check_cancel`, `task_detach`.

**Group operations:**
`group_new`, `group_spawn`, `group_join`, `group_cancel`, `group_close`.

**Channel operations:**
`chan_new`, `chan_send`, `chan_recv`, `chan_try_send`, `chan_try_recv`,
`chan_close`.

**Select operations:**
`select_new`, `select_recv`, `select_send`, `select_join`, `select_timer`,
`select_cancel`, `select_default`, `select_wait`.

**Cancel token:**
`new_cancel_token`.

**Run-loop / inspection:**
`pick_next`, `set_current`, `complete_current`, `fail_current`,
`advance_clock`, `current_time`, `seed`, `is_done`, `has_ready`,
`task_state`.

**Constructors:**
`Scheduler::new()` (production mode), `Scheduler::deterministic(seed)`
(FIFO queue, clock starts at ZERO, select picks lowest ArmId).

#### Tests
- 42 module-level unit tests across all 7 source modules.
- 65 integration tests in 5 test files:
  - `test_task.rs` — 18 tests: spawn, yield, sleep, join, cancel, detach.
  - `test_channel.rs` — 14 tests: bounded, rendezvous, park/wake, close.
  - `test_group.rs` — 8 tests: FailFast, CollectErrors, cancel, close, detach.
  - `test_select.rs` — 9 tests: immediate fire, parking, fairness, timer, cancel.
  - `test_cancel.rs` — 6 tests: token lifecycle, propagation, select arm.
  - `test_deterministic.rs` — 10 tests: FIFO order, clock, seed, wakeup.
- 2 doc-tests (lib.rs quick-start, Deadline API).

**Total: 109 tests, 0 failures.**

---

*Previous versions: none (initial release).*
