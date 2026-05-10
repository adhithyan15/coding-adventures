# LANG28 - VM-native lightweight concurrency

## Overview

LANG28 defines the concurrency model for LANG VM programs.

The VM should natively support lightweight concurrency primitives.  Language
frontends should be able to emit IIR that spawns tasks, yields, waits, joins,
cancels, selects over events, and communicates through channels without first
choosing a host runtime such as pthreads, Win32 threads, Tokio, JVM threads,
CLR tasks, BEAM processes, or Web Workers.

The VM owns those lightweight semantics.  The implementation may multiplex
them onto:

- one interpreter loop;
- a pool of OS threads;
- OS event backends such as `epoll`, `kqueue`, IOCP, or host message loops;
- host VM facilities such as JVM executors, CLR tasks, BEAM processes, or WASM
  event/worker shims;
- child OS processes where isolation or capability boundaries require it.

That multiplexing is an implementation detail.  The language-visible
primitive is a LANG task, not an OS thread.

Real OS threads and real OS processes remain available through `liblang-std`.
Those APIs are explicit, capability-gated host operations.  They are useful for
native interop, process control, shell-like programs, and workloads that must
bind to OS behavior.  They do not replace the VM's portable lightweight task
model.

## Goals

- Add a portable IIR concurrency surface that every frontend can target.
- Keep lightweight tasks cheap enough for languages to use them freely.
- Let the VM multiplex many tasks over a bounded set of OS threads.
- Let the VM optionally place task groups into child processes for isolation.
- Expose real OS threads and processes through `liblang-std`, not as the
  default language concurrency primitive.
- Preserve GC, debugger, profiler, coverage, JIT, AOT, and host VM behavior.
- Make concurrency deterministic under test when requested.
- Make frontend work simple: Twig, Tetrad, Ruby, TypeScript, Lua, Perl,
  Brainfuck, and BASIC should all lower to the same VM concurrency operations.

## Non-goals

- Replacing `event-loop` or `native-event-core`.  LANG28 consumes those layers.
- Making all tasks preemptive at the source-language level on day one.
- Requiring every target to support true parallel execution.
- Exposing raw host synchronization primitives directly through IIR.
- Treating OS process management as portable opcodes.  OS processes are
  `liblang-std` host APIs with capability checks.
- Closed-world AOT specialization of concurrency.  Correct runtime calls come
  first; inlining and specialization can follow.

## Layering

```
language frontend
    emits IIR task/channel/select operations
        |
        v
vm-core / vm-runtime
    owns task state, scheduling, parking, wakeups, cancellation, joins,
    channel queues, debugger hooks, metrics, GC root enumeration
        |
        +--> pure interpreter scheduler
        +--> JIT/AOT runtime calls through liblang-runtime
        +--> native event backends through native-event-core
        +--> host VM adapters through LANG27
        |
        v
liblang-std
    explicit host APIs for std/thread, std/process, std/io, std/net, timers
```

The key rule is simple:

| Layer | Owns |
| --- | --- |
| IIR opcodes | Portable lightweight tasks, channels, cancellation, select |
| VM runtime | Scheduling, multiplexing, task stacks, parking, wakeups |
| `native-event-core` | OS readiness/completion/message backends |
| `liblang-std` | Real OS thread and process APIs |
| Host VM lowerers | Target-specific realization without changing semantics |

## Core model

### Task

A LANG task is a VM-managed unit of execution.

Each task has:

- a stable `TaskId`;
- a task state;
- an interpreter frame stack or compiled-frame continuation;
- a parent task or detached flag;
- an optional task group;
- an optional name for diagnostics;
- a priority hint;
- cancellation state;
- deadline metadata;
- a mailbox/channel wait slot;
- root-scanning metadata for GC;
- debugger/profiler identity.

Task states:

| State | Meaning |
| --- | --- |
| `new` | Allocated but not yet scheduled |
| `ready` | Runnable and queued |
| `running` | Currently executing on a VM worker |
| `parked` | Waiting for a channel, timer, join, I/O event, or debugger pause |
| `cancel_requested` | Cancellation is pending and will be observed at a safepoint |
| `completed` | Returned a value |
| `failed` | Raised a language runtime error or VM trap |
| `cancelled` | Exited through cooperative cancellation |
| `detached` | Completion is not joined by a parent |

Tasks are not OS threads.  Many tasks can run on one OS thread, and many tasks
can be spread over a small worker pool.

### Task group

A task group is a structured-concurrency scope.

Properties:

- child tasks inherit capability context unless explicitly narrowed;
- group exit waits for or cancels remaining children;
- group failure policy is explicit: `fail_fast`, `collect_errors`, or
  `supervise`;
- group cancellation propagates to children;
- debugger and metrics can show group hierarchies.

Detached tasks are allowed, but they must be explicit.  A frontend should not
silently detach work that can outlive its lexical scope.

### Channel

A channel is a VM-managed communication object.

Channel properties:

- element type or `any`;
- bounded or unbounded capacity;
- FIFO ordering per sender;
- close state;
- wait queues for senders and receivers;
- optional deterministic test seed for fair wakeup ordering.

Channels are the first portable communication primitive.  Shared mutable state
can exist, but it should use stdlib locks/atomics or language-level ownership
rules rather than raw VM opcodes.

### Select

`select` waits on multiple operations:

- receive from channel;
- send to channel when capacity exists;
- task join;
- timer/deadline;
- I/O readiness/completion event surfaced by a stdlib async handle;
- cancellation token.

The VM owns fairness.  The deterministic scheduler mode can choose a fixed
ordering; production mode should avoid starvation.

### Cancellation

Cancellation is cooperative.

A task observes cancellation at:

- `task_yield`;
- `task_sleep`;
- channel send/receive/select;
- join;
- VM safepoints from LANG16;
- runtime calls that declare they may park or allocate;
- explicit `task_check_cancel`.

The VM must not asynchronously tear down a task in the middle of arbitrary
language execution.  Native host calls can be marked `uncancellable`,
`deferred_cancel`, or `host_cancel`.

### Memory model

VM-managed code must not expose data races.

Rules:

- immutable values can be shared freely;
- channel transfer preserves language value semantics;
- mutable heap objects shared between tasks require a language/runtime policy:
  ownership transfer, lock, atomic cell, actor/mailbox, or explicit unsafe FFI;
- GC roots are per task and per parked continuation;
- compiled frames must expose stack maps at every parking or safepoint location;
- FFI values crossing OS thread boundaries must satisfy the language binding's
  send/share constraints.

This gives dynamic languages a safe default while still letting low-level
frontends opt into explicit shared state.

## IIR additions

### New value kinds

| Kind | Description |
| --- | --- |
| `task<T>` | Handle to a VM task that returns `T` |
| `task_group` | Structured-concurrency scope |
| `channel<T>` | VM channel carrying values of type `T` |
| `select_set` | Builder/runtime object for one select operation |
| `cancel_token` | Cancellation authority or child token |
| `deadline` | Monotonic deadline/timer value |
| `os_thread` | Opaque stdlib handle, not a VM task |
| `process_handle` | Opaque stdlib child-process handle |

The last two kinds are included so the type system can model stdlib host APIs.
They are not produced by lightweight task opcodes.

### Lightweight task opcodes

| Mnemonic | Dest | Operands | Description | May park |
| --- | --- | --- | --- | --- |
| `task_spawn` | `task<T>` | `(fn, args..., options)` | Spawn a VM task | no |
| `task_current` | `task<any>` | `()` | Current task handle | no |
| `task_yield` | none | `()` | Yield to the scheduler and observe cancellation | yes |
| `task_sleep` | none | `(deadline)` | Park until deadline or cancellation | yes |
| `task_join` | `result<T>` | `(task)` | Wait for task completion | yes |
| `task_cancel` | `bool` | `(task, reason)` | Request cooperative cancellation | no |
| `task_check_cancel` | none | `()` | Trap/raise if cancellation is pending | no |
| `task_detach` | none | `(task)` | Mark task completion as unjoined | no |

### Task group opcodes

| Mnemonic | Dest | Operands | Description | May park |
| --- | --- | --- | --- | --- |
| `group_new` | `task_group` | `(policy)` | Create structured task scope | no |
| `group_spawn` | `task<T>` | `(group, fn, args..., options)` | Spawn child task | no |
| `group_join` | `result<list<any>>` | `(group)` | Wait for all children | yes |
| `group_cancel` | none | `(group, reason)` | Cancel all children | no |
| `group_close` | none | `(group)` | End scope; fail if live detached children remain | yes |

### Channel opcodes

| Mnemonic | Dest | Operands | Description | May park |
| --- | --- | --- | --- | --- |
| `chan_new` | `channel<T>` | `(capacity, type_id)` | Create channel | no |
| `chan_send` | `bool` | `(channel, value)` | Send or park until capacity | yes |
| `chan_recv` | `option<T>` | `(channel)` | Receive or park until value/close | yes |
| `chan_try_send` | `bool` | `(channel, value)` | Non-blocking send | no |
| `chan_try_recv` | `option<T>` | `(channel)` | Non-blocking receive | no |
| `chan_close` | none | `(channel)` | Close channel and wake waiters | no |

### Select opcodes

| Mnemonic | Dest | Operands | Description | May park |
| --- | --- | --- | --- | --- |
| `select_new` | `select_set` | `()` | Start a select builder | no |
| `select_recv` | none | `(set, channel, tag)` | Add receive case | no |
| `select_send` | none | `(set, channel, value, tag)` | Add send case | no |
| `select_join` | none | `(set, task, tag)` | Add task join case | no |
| `select_timer` | none | `(set, deadline, tag)` | Add timer case | no |
| `select_cancel` | none | `(set, token, tag)` | Add cancellation case | no |
| `select_wait` | `select_result` | `(set)` | Park until one case wins | yes |

`select_result` contains the winning tag, operation kind, optional value, and
status such as `ready`, `closed`, `cancelled`, or `timed_out`.

### Why process and thread are not opcodes

The VM needs native lightweight concurrency everywhere, including embedded,
WASM, BEAM, test interpreters, and educational targets.  OS threads and OS
processes do not have portable creation, scheduling, permission, environment,
stdio, signal, and cleanup semantics.

Therefore:

- `task_spawn` creates VM tasks;
- `std/thread/spawn` creates real OS threads when the target supports them;
- `std/process/spawn` creates real child processes when capability grants allow
  it;
- OS handles are opaque stdlib values;
- the VM may internally run tasks on OS threads or processes, but IIR programs
  do not depend on that mapping.

## Runtime scheduler

### First implementation

The first implementation should be a single-threaded cooperative scheduler in
`vm-core` and `vm-runtime`.

It needs:

- task table;
- ready queue;
- parked wait lists;
- timer heap;
- channel queues;
- join waiters;
- cancellation tokens;
- deterministic scheduling mode for tests;
- debugger hooks for task create, park, wake, complete, fail, and cancel;
- metrics hooks for LANG17.

This makes semantics testable before OS parallelism enters the picture.

### M:N implementation

The production scheduler should support M:N scheduling:

```
many VM tasks
    |
    v
bounded VM worker pool
    |
    +--> OS thread 0
    +--> OS thread 1
    +--> OS thread N
```

Requirements:

- global inject queue for new tasks;
- per-worker local queues for cache locality;
- work stealing for load balance;
- backpressure when task or channel limits are reached;
- cooperative safepoints for fairness;
- optional instruction-budget preemption in interpreter mode;
- parking without blocking worker threads;
- worker shutdown and panic containment.

The existing `generic-job-runtime` and JR01 thread-pool work can provide
bounded worker-pool patterns, cancellation result shapes, panic containment,
timeouts, and metrics.  LANG28 should not make that crate VM-specific; it
should reuse the proven policies.

### Process-backed execution

The VM may place a task group into one or more child processes for isolation,
resource control, or host-language bridge execution.

This is an execution policy, not a different language primitive:

- task handles remain `task<T>`;
- messages cross process boundaries through serialized VM values;
- process lifecycle is tracked through `liblang-std` and `native-event-core`;
- failures are reported as task failures with structured process details;
- debugger support requires a proxy sidecar per child process.

Process-backed execution is optional for the initial implementation.

## Integration points

### GC and safepoints

LANG16 root scanning must become task-aware.

The collector must see:

- running task frames;
- parked continuations;
- channel buffers;
- select builders;
- join results not yet consumed;
- timer payloads;
- stdlib async handles that retain VM values.

Every parking point is a safepoint.  JIT and AOT code must provide stack maps
for parking points exactly as they do for allocation/deopt safepoints.

### Debugger

The debugger must present concurrency as first-class state:

- list tasks and task groups;
- show each task state;
- show the current frame stack for running and parked tasks;
- indicate park reasons: channel, timer, join, I/O, debugger pause;
- support breakpoints that pause one task or all tasks by policy;
- support stepping within a selected task;
- support deterministic replay mode once event traces exist;
- report cancellation and task failure as structured events.

This directly extends LANG06/LANG13/LANG25 debugger work.  A debugger that only
shows one process-global stack is not sufficient once LANG28 exists.

### Profiling, metrics, and coverage

LANG17 and LANG18 should gain task dimensions:

- task count by state;
- scheduler queue depth;
- park/wake counts by reason;
- channel send/receive latency;
- select wake distribution;
- cancellation latency;
- worker utilization;
- per-task coverage attribution where practical.

The scheduler must avoid making metrics collection a global lock bottleneck.

### JIT and AOT

Initial lowering:

- task/channel/select opcodes lower to `liblang-runtime` calls;
- every may-park runtime call is a safepoint;
- compiled frames provide stack maps at may-park calls;
- compiled tasks can deopt back into interpreter frames before parking if the
  backend cannot preserve a resumable continuation yet.

Later lowering:

- inline fast-path `chan_try_send` and `chan_try_recv`;
- inline cancellation checks;
- specialize task-spawn closures with known signatures;
- elide scheduler calls in proven single-task regions;
- use PGO to choose queue and worker placement hints.

Correctness comes before cleverness.  Runtime-call lowering is enough for the
first JIT/AOT path.

### Host VM backends

LANG27 host VM lowering must preserve LANG28 semantics.

| Target | First mapping | Notes |
| --- | --- | --- |
| Pure LANG VM | VM scheduler directly | Reference behavior |
| JVM | Runtime scheduler using Java host threads/executors as workers | Do not require Java thread per LANG task |
| CLR | Runtime scheduler using ThreadPool/Task infrastructure as workers | Preserve VM cancellation semantics |
| BEAM | Prefer BEAM process/mailbox mapping where it matches | Fall back to runtime scheduler for unsupported features |
| WASM | Single-thread event-loop scheduler first | Add Web Worker/WASI thread support later |

Host facilities can improve implementation quality, but they must not leak
different semantics to frontends.

### Native event backends

LANG28 consumes `native-event-core` for OS wakeups:

- Linux: `epoll` first, `io_uring` later;
- macOS/BSD: `kqueue`;
- Windows: IOCP for overlapped I/O and Win32 message loops for UI events;
- UI platforms: platform message loops are separate event sources;
- timers and process lifecycle events are normalized into VM wakeups.

Blocking OS calls must not block a VM worker unless the runtime deliberately
marks the operation as a blocking host section and compensates with another
worker.

### Shared stdlib

LANG26 should expose explicit host APIs:

| Std module | Purpose |
| --- | --- |
| `std/task` | Friendly wrappers over VM task opcodes |
| `std/channel` | Friendly wrappers over VM channel opcodes |
| `std/select` | Friendly wrappers over VM select opcodes |
| `std/thread` | Real OS threads, capability-gated |
| `std/process` | Real child processes, capability-gated |
| `std/sync` | Mutexes, semaphores, atomics, once cells |
| `std/time` | Monotonic deadlines and sleeps |
| `std/io/async` | Async handles that park VM tasks |

`std/thread` and `std/process` must declare effects and capabilities.  Tests
can deny them while still allowing VM tasks/channels.

## Capability model

Lightweight VM tasks are usually safe to grant by default, subject to runtime
resource limits.

Host APIs require explicit capabilities:

- `thread.spawn`;
- `thread.set-priority`;
- `process.spawn`;
- `process.signal`;
- `process.env`;
- `process.cwd`;
- `process.stdio.inherit`;
- `process.stdio.pipe`;
- `network.open`;
- `filesystem.open`.

AOT binaries embed the capability manifest.  JIT/interpreter sessions carry it
in runtime context.  Host VM artifacts must preserve the same checks.

## Deterministic test mode

The scheduler must support deterministic test execution.

In deterministic mode:

- task IDs are allocated predictably;
- ready queues use stable ordering;
- select tie-breaking is seeded and reproducible;
- timers advance through a virtual clock unless real time is requested;
- I/O completions can be injected as scripted events;
- random work stealing is disabled;
- process/thread stdlib calls can be denied or backed by test doubles.

Every language frontend should be able to run concurrency conformance tests
without racing the host operating system.

## Language frontend guidance

### Twig

Twig should expose structured task scopes and typed channels.  The refined type
system can prove channel element constraints, non-empty receives after guarded
selects, and cancellation-safe resource scopes.

### Tetrad

Tetrad should prefer statically typed task return values and channel element
types.  The compiler should reject sending values that do not satisfy the
channel's declared type/refinement.

### Ruby, TypeScript, Lua, and Perl

Dynamic languages can lower their surface async/thread/fiber/coroutine
features to VM tasks where semantics match.  Their runtimes keep language
specific behavior for exceptions, promises, fibers, coroutines, and method
dispatch, but execution uses LANG28 primitives.

### Brainfuck and BASIC

These languages do not need rich concurrency initially, but their I/O should
use stdlib handles that can park VM tasks.  This keeps them compatible with the
same scheduler and debugger.

## Migration plan

### 28A - Spec and opcode model

- Land this spec.
- Add opcode names and type kinds to `interpreter-ir`.
- Add parser/serializer support for the new opcodes.
- Add static validation rules: may-park markers, result shapes, and channel
  element types.

### 28B - Single-thread VM scheduler

- Implement task table, ready queue, timers, channels, joins, cancellation.
- Run interpreter-only conformance tests.
- Add deterministic scheduler mode.
- Add basic metrics.

### 28C - Debugger and GC integration

- Make root enumeration task-aware.
- Add debugger task list and per-task stack inspection.
- Emit task lifecycle events through the debug sidecar.
- Add breakpoint tests with multiple tasks.

### 28D - Native event integration

- Connect timers and async I/O handles to `native-event-core`.
- Ensure parking does not block VM workers.
- Add Linux/macOS/Windows smoke tests where host support exists.

### 28E - M:N worker pool

- Add bounded worker pool execution.
- Add work stealing and blocking-section compensation.
- Add panic/trap containment.
- Add stress tests for fairness and cancellation.

### 28F - Stdlib OS thread/process APIs

- Add `std/thread` and `std/process` manifests with capabilities.
- Route child-process lifecycle events into VM wakeups.
- Add deterministic denied-by-default host adapters for tests.

### 28G - JIT, AOT, and host VM lowering

- Lower may-park operations to runtime calls with stack maps.
- Add deopt-before-park fallback for backends that need it.
- Map LANG28 semantics through JVM, CLR, BEAM, and WASM host backends.
- Add conformance tests that compare pure VM, JIT, AOT runtime fallback, and
  host VM execution.

## Definition of done

LANG28 is complete when:

- a frontend can emit IIR with tasks, channels, joins, cancellation, timers,
  and select;
- the pure VM runs those programs deterministically in test mode;
- GC sees all roots in running and parked tasks;
- the debugger can inspect and step a selected task;
- JIT and AOT paths can execute may-park operations through runtime calls;
- host VM backends preserve semantics for the shared conformance suite;
- `liblang-std` exposes real OS thread and process APIs with capabilities;
- denied capabilities fail before host side effects occur;
- task-heavy programs do not require one OS thread per task;
- all language ports can reuse the same concurrency conformance tests.

## Open questions

- Should task priority be a portable semantic guarantee or only a scheduler
  hint?
- Should actor mailboxes be a first-class VM value or a stdlib layer over
  channels?
- How much preemption should interpreter mode provide beyond safepoint budgets?
- Which host VM targets should be required for the first cross-target
  conformance gate?
- Should process-backed task groups serialize values through the same format as
  host VM artifacts or through a dedicated runtime message format?
