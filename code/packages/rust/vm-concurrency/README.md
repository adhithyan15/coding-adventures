# vm-concurrency

**Single-thread cooperative scheduler for the LANG VM (LANG28B).**

This crate is the runtime backbone for the 27 concurrency opcodes introduced in
`interpreter-ir` v0.3.0 (LANG28A).  It implements structured concurrency —
tasks, channels, groups, select, and cancel tokens — on a single OS thread
using cooperative yielding.

---

## Where this fits

```
bytecode stream  ──►  interpreter-ir (IIRInstr)
                              │
                              ▼
                    vm-core dispatch loop
                              │
                              ▼
                    vm-concurrency Scheduler
                    ┌─────────────────────────────────┐
                    │  task table   (TaskEntry)        │
                    │  ready queue  (VecDeque)         │
                    │  timer heap   (BinaryHeap)       │
                    │  channels     (ChannelEntry)     │
                    │  groups       (GroupEntry)       │
                    │  select sets  (SelectSetEntry)   │
                    │  cancel tokens(CancelTokenEntry) │
                    └─────────────────────────────────┘
```

The scheduler owns all concurrency state.  It is **single-threaded**; all
methods take `&mut self`.  There is no `Send`/`Sync` requirement — everything
runs on the calling thread.

---

## Quick start

```rust
use vm_concurrency::{Scheduler, Value};

// Create a deterministic scheduler (stable test ordering, clock at zero).
let mut sched = Scheduler::deterministic(0);

// Spawn two tasks (just handles — you supply the execution logic).
let t1 = sched.task_spawn().unwrap();
let t2 = sched.task_spawn().unwrap();

// Run-loop: pick → set_current → execute → complete/fail/park.
let next = sched.pick_next().unwrap();  // → t1 (FIFO)
sched.set_current(next);
// ... execute instructions for t1 ...
sched.complete_current(Value::Int(0)).unwrap();

let next = sched.pick_next().unwrap();  // → t2
sched.set_current(next);
sched.complete_current(Value::Bool(true)).unwrap();

assert!(sched.is_done());
```

---

## Key types

| Type | Description |
|------|-------------|
| `TaskHandle` | Opaque `u32` handle to a task |
| `ChannelId` | Bounded message queue |
| `GroupId` | Structured concurrency scope |
| `SelectSetId` | One in-flight `select` operation |
| `CancelTokenId` | Cooperative cancellation token |
| `Deadline` | Monotonic nanosecond timestamp |
| `Value` | Re-export of `vm_core::value::Value` |

---

## Task lifecycle

```
spawn ──► Ready ──► Running ──► Completed
                       │          Failed
                       │          Cancelled
                       └──► Parked(reason)
                               │
                         wake (chan/join/timer/cancel/group)
                               │
                             Ready ──► Running …
```

`Parked` wraps a `ParkReason` describing why the task is waiting:

| ParkReason | Waiting for |
|------------|-------------|
| `Yield` | Back of the ready queue |
| `Sleep(deadline)` | `advance_clock(t)` where `t >= deadline` |
| `Join(task)` | Target task to complete or fail |
| `ChanSend(ch)` | Buffer space to open up |
| `ChanRecv(ch)` | A value to arrive (or channel close) |
| `GroupJoin(group)` | All non-detached children to finish |
| `Select(set)` | Any arm in the select set to fire |

---

## Channels

Channels are created with `chan_new(capacity)`:

- **capacity > 0** — buffered channel; senders park when the buffer is full,
  receivers park when it is empty.
- **capacity == 0** — rendezvous channel; sender parks until a receiver is
  waiting (synchronised handoff, zero buffering).

```rust
let ch = sched.chan_new(2).unwrap();      // buffered, cap 2
sched.chan_send(ch, Value::Int(1)).unwrap(); // → Sent (space available)
sched.chan_send(ch, Value::Int(2)).unwrap(); // → Sent
// sched.chan_send(ch, Value::Int(3))     // → Parked (buffer full)
```

---

## Task groups

Groups provide structured concurrency: a parent waits for all children to
finish before proceeding.

```rust
let group = sched.group_new(GroupPolicy::FailFast).unwrap();
let child = sched.group_spawn(group).unwrap();

// ... child runs and completes ...
let result = sched.group_join(group).unwrap(); // parks until all children done
```

`GroupPolicy` controls what happens when a child fails:

| Policy | Behaviour |
|--------|-----------|
| `FailFast` | First failure cancels all siblings immediately |
| `CollectErrors` | Wait for all children; collect every error |
| `Supervise` | Caller decides (errors accessible via `group_join`) |

---

## Select

`select` waits for the first of several events:

```rust
let set = sched.select_new().unwrap();
let recv_arm = sched.select_recv(set, ch1).unwrap();
let send_arm = sched.select_send(set, ch2, Value::Int(9)).unwrap();
sched.select_default(set).unwrap();

match sched.select_wait(set).unwrap() {
    None => { /* parked — will be woken when an arm fires */ }
    Some(result) => {
        // result.arm_id   → which arm fired
        // result.kind     → Recv / Send / Join / Timer / Cancel / Default
        // result.value    → received value (Recv/Join arms only)
        // result.status   → Ready / Closed / Cancelled / TimedOut / Default
    }
}
```

Arm kinds: `Recv`, `Send`, `Join`, `Timer`, `Cancel`, `Default`.

In deterministic mode tie-breaking always picks the arm with the **lowest
`ArmId`** (arms are numbered 0, 1, 2, … in add order).

---

## Cooperative cancellation

Cancel tokens let one task signal another to stop:

```rust
let token = sched.new_cancel_token().unwrap();

// Canceller task:
sched.task_cancel(target, token).unwrap();

// Target task polls at safepoints:
if sched.task_check_cancel().unwrap() {
    // clean up and return early
}
```

`task_check_cancel` is a **consume-once** check: it returns `true` once and
clears the flag.

A `select_cancel(set, token)` arm fires immediately if the token is already
cancelled; otherwise it parks until cancelled.

---

## Deterministic mode

`Scheduler::deterministic(seed)` is designed for test suites:

- Tasks run in **FIFO order** (spawn order).
- The virtual **clock starts at `Deadline::ZERO`** and never advances on its
  own.  Call `advance_clock(t)` to wake sleeping tasks.
- Select tie-breaking picks the arm with the **lowest `ArmId`** (seeded RNG
  planned for future LANG28C).
- `current_time()` and `seed()` are available for assertions.

---

## Design notes

### No executor thread
The scheduler has no internal thread.  The `vm-core` dispatch loop *is* the
"executor" — it calls `pick_next` / `set_current`, interprets instructions,
and calls `complete_current` / `fail_current`.

### Two-phase select resolution
`select_wait` scans all arms synchronously.  If any arm can fire immediately
it resolves in one call.  Otherwise the set is stored and the current task is
parked under `ParkReason::Select`; an external event (channel send, task
complete, timer, cancel) calls back into the scheduler to resolve the set and
re-enqueue the waiter.

### FIFO skip logic in `pick_next`
Tasks may be in the ready queue in a non-`Ready` state (e.g. completed while
another task was running).  `pick_next` skips such stale entries rather than
panicking.

---

## Crate dependencies

- `interpreter-ir` — for future opcode dispatch (not yet used in v0.1).
- `vm-core` — provides `Value`.

---

## Testing

```bash
cargo test -p vm-concurrency
```

109 tests across 5 integration test files and 7 unit test modules.
