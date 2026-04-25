# Hardware VM (Event-Driven Simulator)

## Overview

Hardware is parallel by default. A Verilog `always @(posedge clk)` and a VHDL `process(clk)` model two processes that *both* run when `clk` rises — concurrently, in zero time, until they hit their next suspension point. Modeling this on a sequential CPU requires an **event-driven simulator with delta cycles**: every signal update is an event timestamped `(t, δ)`; events are drained in order; processes wake up when their sensitivity list fires; updates from a delta cycle become visible only at the next delta. This is the IEEE 1364 §11 / IEEE 1076 §10.5 reference model, and it is what we implement here.

The Hardware VM consumes HIR (`hdl-ir.md`), runs simulation, and emits events to consumers (waveform writers, coverage instruments, testbench checkers). It does not care which front-end produced the HIR; it does not care whether the design is a 4-bit adder or a million-cell SoC — only the runtime constants change.

### Design choices

- **Event-driven scheduler** (not lock-step). Hardware sim is sparse-update: most signals don't change every cycle. Iterating only over wakeups gives orders of magnitude speedup over evaluating every process every cycle.
- **Delta cycles** for parallel-by-default semantics. At time `t`, all processes whose sensitivity fires run "simultaneously"; their updates become visible at the next delta within the same `t`. Time advances only when no more deltas are pending at `t`.
- **CPS-style process interpreter** (continuation-passing). `wait`, `@`, and `#delay` are continuation save points. A process is a stack of resumable closures. This is cleaner than tree-walking interpreters that have to special-case every suspension construct.
- **Generators as continuations** (Python implementation). Python `yield` is a perfect fit: each suspension yields control back to the scheduler.
- **4-state and 9-state value semantics**. `Logic` (4-state: `0/1/X/Z`) for Verilog default; `StdLogic` (9-state per IEEE 1164: `U/X/0/1/Z/W/L/H/-`) for VHDL with `std_logic_1164`. Resolution functions when multiple drivers share a net.

## Layer Position

```
HIR (hdl-ir.md)
    │
    ▼
hardware-vm.md  ◀── THIS SPEC
    │ (event-driven kernel)
    │
    ├──► VCD events ──► vcd-writer.md
    ├──► coverage events ──► coverage.md
    └──► assertions / testbench ──► testbench-framework.md
```

## Concepts

### The simulation kernel

The kernel maintains:

1. **Event queue** — a min-heap of pending updates ordered by `(time, delta, sequence)`.
2. **Signal table** — every Net's current value, pending updates, and list of sensitive processes.
3. **Process pool** — for each Process in HIR: an active continuation (suspended somewhere in its body) plus its sensitivity set.
4. **Time** — a 64-bit integer counting picoseconds (or whatever the timescale is).
5. **Delta** — a counter, reset to 0 each time `time` advances.

### The simulation cycle

```
while event_queue not empty and time <= sim_end:
  next_time, next_delta = peek(event_queue)
  if next_time > time:
    advance time = next_time, delta = 0
  else:
    delta += 1
  
  # Phase 1: drain all events at (time, delta-1) updates → signals
  while peek_event_time_delta() == (time, delta - 1):
    event = pop()
    apply_to_signal(event)
  
  # Phase 2: wake up processes whose sensitivity fired
  for net in nets_with_pending_updates_committed_this_delta:
    for process in net.sensitive_processes:
      mark_runnable(process)
  
  # Phase 3: run all runnable processes; collect their new updates
  while runnable_processes:
    p = runnable_processes.pop()
    advance_continuation(p)   # runs until next yield/wait
  
  # Phase 4: schedule any newly produced events at (time, delta)
  # (already done as side effect of process execution)

  # If no more events at this time, advance.
```

### Blocking vs non-blocking assignment

Verilog has two: `=` (blocking — immediate) and `<=` (non-blocking — deferred). The textbook footgun:

```verilog
always @(posedge clk) begin
  a <= b;      // a is updated AT END of delta, with value of b NOW
  b <= a;      // b is updated AT END of delta, with value of a NOW
end
// After: a and b swap values.
```

vs:

```verilog
always @(posedge clk) begin
  a = b;       // a := b immediately
  b = a;       // b := a (which is now b)
end
// After: both = b (original).
```

The kernel models this:
- Blocking assigns happen *during process execution* — they update the signal table immediately and are visible to subsequent statements in the same process.
- Non-blocking assigns are scheduled at `(time, delta+1)` — they happen at the start of the next delta and are visible to *all* processes in the next delta.

VHDL signal `<=` is non-blocking (default schedule at next delta); VHDL variable `:=` is blocking (immediate).

### 4-state and 9-state values

A `Logic` value is one of: `'0'`, `'1'`, `'X'` (unknown), `'Z'` (high-impedance). Operators on Logic produce Logic per truth tables that are extended to handle X and Z:

```
   AND   0  1  X  Z         OR   0  1  X  Z         NOT
    0    0  0  0  0           0    0  1  X  X           0 → 1
    1    0  1  X  X           1    1  1  1  1           1 → 0
    X    0  X  X  X           X    X  1  X  X           X → X
    Z    0  X  X  X           Z    X  1  X  X           Z → X
```

`StdLogic` (IEEE 1164) extends to 9 states: `U/X/0/1/Z/W/L/H/-` with formal resolution rules for multi-driver nets.

A `StdLogicVector` resolves bit-by-bit.

### Resolution functions

When a net has multiple drivers (e.g., a tristate bus), the resolution function combines them:

| All drivers | Resolved value |
|---|---|
| All 'Z' | 'Z' |
| Single driver, others 'Z' | The single driver |
| Multiple non-Z drivers, conflict | 'X' |
| `wand`/`wor` | AND/OR of the drivers |

The simulator looks up the resolution function from the net's `kind` and invokes it whenever any driver changes.

### Sensitivity firing

A process's sensitivity list specifies which net changes wake it:
- `posedge clk`: fires when `clk` transitions from 0/X/Z to 1.
- `negedge clk`: fires when `clk` transitions from 1/X/Z to 0.
- `change x`: fires on any value change.

The simulator records each net's previous value; when a delta commits an update, it checks each sensitive process and queues those whose edge condition matches.

### Wait statements as continuations

A Verilog process:
```verilog
initial begin
  a = 0;
  #10;
  a = 1;
  @(posedge clk);
  a = b;
end
```

Compiles to a generator-style coroutine:
```python
def proc():
    a.set(0)
    yield ("delay", 10)
    a.set(1)
    yield ("event", clk.posedge)
    a.set(b.value)
    # falls off end → process terminates
```

The scheduler calls `next(proc())` to advance until the next `yield`, reads the suspension reason, and either schedules a wakeup at `t+10` or registers `proc` as sensitive to `clk.posedge`.

VHDL:
```vhdl
process
begin
  a <= '0';
  wait for 10 ns;
  a <= '1';
  wait until rising_edge(clk);
  a <= b;
  wait;
end process;
```
maps to the same shape.

### Initial vs always

- Verilog `initial` runs once at time 0.
- Verilog `always @(...)` runs whenever sensitivity fires, then re-suspends at top.
- VHDL `process(...)` is the same as `always @(...)` — re-runs from top after each completion.
- VHDL `process` with explicit `wait` runs once and suspends at each `wait`.

The kernel models all four as a unified `Process` whose continuation either falls off the end (one-shot) or returns to the top (re-trigger).

### Force / release

Verilog `force x = expr` overrides any normal driver of `x` until `release x` removes the override. Modeled as a "force shadow" on the signal — when reading, the force value wins.

### Time and timescale

Simulation time is an integer (picoseconds default). `timescale` directive scales: `1ns/100ps` means user-written `#5` is 5 ns of simulated time, with internal precision of 100 ps.

`$time` returns current simulation time.

## VCD Event Emission

The simulator emits events for waveform writers (see `vcd-writer.md`):

```python
class Event(Protocol):
    time: int
    kind: str   # 'value_change', 'process_start', 'assert_failure'
    payload: dict
```

The VCD writer subscribes to `value_change` events and produces VCD output. The coverage instrument subscribes too. Multiple consumers; one event stream.

## Public API

```python
from dataclasses import dataclass, field
from enum import Enum
from typing import Generator, Protocol


class LogicValue(Enum):
    ZERO = "0"
    ONE  = "1"
    X    = "X"
    Z    = "Z"


@dataclass
class SignalState:
    value: object       # LogicValue, int, list[LogicValue], etc.
    drivers: dict[str, object]   # process_name → driven_value
    sensitive_processes: set[str]
    pending_event: object | None = None
    last_value: object | None = None    # for edge detection
    forced: object | None = None        # active force, if any


@dataclass
class Process:
    name: str
    body: Generator              # the continuation
    sensitivity: list["Sensitivity"]
    runnable: bool = False
    next_resume: object = None   # event/value to feed the generator


@dataclass(frozen=True)
class Sensitivity:
    net: str
    edge: str   # 'posedge' | 'negedge' | 'change'


@dataclass
class ScheduledEvent:
    time: int
    delta: int
    seq: int
    target_net: str
    new_value: object
    schedule_kind: str   # 'nb' (non-blocking) | 'cont' (continuation wakeup)


class HardwareVM:
    def __init__(self, hir: "HIR", timescale: int = 1):
        ...
    
    def run(self, until_time: int) -> "RunResult":
        ...
    
    def step(self) -> bool:
        """Run one delta cycle. Returns True if more events remain."""
        ...
    
    def force(self, signal: str, value: object) -> None: ...
    def release(self, signal: str) -> None: ...
    def deposit(self, signal: str, value: object) -> None: ...
    def read(self, signal: str) -> object: ...
    
    def subscribe(self, kind: str, callback) -> None: ...
    
    @property
    def time(self) -> int: ...
    @property
    def delta(self) -> int: ...


@dataclass
class RunResult:
    final_time: int
    process_count: int
    event_count: int
    assertions_failed: int
```

## Worked Example 1 — 4-bit Adder Simulation

HIR has one Module `adder4` with one ContAssign: `{cout, sum} = a + b + cin`.

Testbench applies stimulus:
```verilog
initial begin
  a = 4'b0001; b = 4'b0010; cin = 0;
  #10 a = 4'b0111; b = 4'b1001; cin = 1;
  #10 $finish;
end
```

VM execution:
- `t=0, δ=0`: initial process runs to `#10`. Sets a, b, cin (blocking assigns → immediate). ContAssign on `sum/cout` re-evaluates because `a/b/cin` changed; schedules NBA at `(0, δ+1)`.
- `t=0, δ=1`: NBA delivered: `sum=00011, cout=0`. Emits VCD events.
- `t=10, δ=0`: initial process resumes. Updates a, b, cin. ContAssign re-evaluates.
- `t=10, δ=1`: `sum=0000, cout=1` (overflow). VCD events emitted.
- `t=20`: `$finish` called; sim ends.

Output VCD shows the value transitions; testbench can check.

## Worked Example 2 — Race-condition test (blocking vs non-blocking)

```verilog
reg [1:0] a, b;
initial a = 2'b00;
initial b = 2'b00;

always @(posedge clk) a = b + 1;
always @(posedge clk) b = a + 1;
```

vs

```verilog
always @(posedge clk) a <= b + 1;
always @(posedge clk) b <= a + 1;
```

Blocking version: order matters and is unspecified by Verilog. The two processes can race; result depends on scheduler order.

Non-blocking version: deterministic. At `(t, δ+1)`, both updates apply simultaneously. With `a=0, b=0` initially, after the posedge: `a=1, b=1`.

Our scheduler: warns on order-dependent blocking assigns to shared variables (a known Verilog antipattern), passes deterministically for non-blocking.

## Worked Example 3 — Mid-scale: pipelined ALU testbench

A 32-bit ALU with 4 pipeline stages, ~10K gates after synthesis. Behavioral HIR with ~20 always blocks. Testbench stimulus over 100K cycles. VCD output ~10 MB. Simulation rate target: ~1 µs of sim time per ms of wall time on a modern laptop (i.e., 1000× slowdown vs real hardware — typical for behavioral simulators in Python). For faster simulation, the kernel can compile HIR processes to bytecode (future work), reaching ~10× speedup.

## Edge Cases

| Scenario | Handling |
|---|---|
| Combinational loop without delay | Detected after N delta cycles at same time without progress; halt with error. |
| Process never suspends | Same — infinite loop in zero time → halt. |
| Multiple drivers on a non-resolved net | Error at elaboration (HIR rule H7). |
| Multiple drivers on a resolved std_logic | Resolution function runs; produces 'X' on conflict. |
| `force` and normal driver disagree | Force wins. |
| Initial process at t<0 | Allowed; scheduled at simulation start. |
| `wait for 0 ns` | Becomes a delta-cycle suspension. |
| Time wrap-around | 64-bit time is 2.3 quintillion picoseconds; sufficient. |
| Disable on running process | Process is killed; its pending NBA events are removed. |
| Function call to undefined function | Elaboration error; not a runtime concern. |
| `$finish` from within an always block | Sim ends cleanly. |
| Recursive task call exceeding stack depth | Python recursion limit; flag at hit. |
| File I/O (`$readmemh`, textio) | Performed at runtime; warn on missing file. |

## Test Strategy

### Unit (target 95%+)
- Event queue ordering: events at same time, different deltas, fire in delta order.
- Sensitivity firing: a posedge event triggers all `posedge` sensitive processes.
- Blocking vs non-blocking semantics on a single-process toy example.
- Force/release.
- Delta-cycle convergence on a simple feedback (FF + combinational).
- 4-state and 9-state truth tables.
- Resolution function: 'Z' + '1' = '1'; '1' + '0' = 'X'; etc.

### Integration
- 4-bit adder: matches expected output bit-for-bit on 256 input combinations.
- 32-bit ALU: matches against hand-computed reference for arithmetic, logic, shifts.
- ARM1 reference (`arm1-gatelevel`): runs a small program, registers match expected.
- VHDL textbook examples (FSMs, counters, FIFO): match published outputs.

### Property
- Determinism: same HIR, same testbench → same VCD output.
- Idempotence on save/restore (future feature).
- Performance scaling: linear in number of triggered events.

## Conformance Matrix

| Standard / construct | Coverage |
|---|---|
| **IEEE 1364-2005** event regions | Full (active, inactive, NBA, monitor regions) |
| **IEEE 1076-2008** simulation cycle | Full |
| **IEEE 1164** std_logic resolution | Full |
| Blocking / non-blocking assignment | Full |
| Force / release | Full |
| `wait`, `@`, `#delay` | Full |
| Delta cycles | Full |
| Timing checks (`$setup`, `$hold`) | Recorded; not enforced in v1 |
| SDF back-annotation | Future spec |
| AMS coupling to SPICE | Future spec |
| `initial`, `always`, `process` | Full |
| Disable | Full |
| File I/O | Full |
| Mixed-language designs | Full (VHDL + Verilog hybrid) |

## Open Questions

1. **Bytecode vs tree-walk** — Python generators give us tree-walk for free with reasonable performance for educational designs. Bytecode VM is a future optimization.
2. **Multi-threaded scheduling** — process bodies could in principle run in parallel within a delta. Defer; complexity likely outweighs gain in Python.
3. **Mixed-signal bridge to SPICE** — needs careful timestep coordination; future spec.
4. **Save/restore** — checkpoint and resume long simulations? Defer.
5. **Process priorities** — Verilog 1364 has region semantics (active, inactive, etc.). We model active and NBA; monitor region (`$monitor` ordering) deferred.

## Future Work

- HIR-to-bytecode compiler for 10×+ speedup.
- Multi-process simulation (one process per OS thread for embarrassingly parallel cases).
- Mixed-signal AMS bridge to `spice-engine.md`.
- SDF back-annotation for delay-aware simulation.
- Checkpoint and resume.
- GPU-accelerated cycle-based simulator (alternative engine for cycle-accurate sims).
- Symbolic simulation for formal verification.
