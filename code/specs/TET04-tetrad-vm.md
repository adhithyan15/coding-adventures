# TET04 — Tetrad Register VM Specification

> **🪦 RETIRED (2026-04-25)** — the `tetrad-vm` package has been
> deleted.  Its responsibilities are fully covered by:
>
> - **Execution** — `vm_core.VMCore` (spec [LANG02](LANG02-vm-core.md))
>   configured with `u8_wrap=True`, `max_frames=4`, and a small
>   Tetrad opcode extension (`tetrad.move`).
> - **Metrics surface** — `vm_core` exposes the same V8 Ignition-style
>   feedback-slot state machine, branch counters, loop-iteration
>   counters, and `execute_traced` opt-in tracing (spec
>   [LANG17](LANG17-vm-core-metrics.md)).
> - **Tetrad-shaped wrappers** — `tetrad_runtime.TetradRuntime` mirrors
>   every legacy `TetradVM.*` metric API (`hot_functions`,
>   `feedback_vector`, `type_profile`, `branch_profile`,
>   `loop_iterations`, `call_site_shape`, `execute_traced`,
>   `reset_metrics`) with the same return-type signatures, so callers
>   can switch `TetradVM` → `TetradRuntime` without rewriting
>   metric-reading code.
>
> The text below is preserved as a historical record of what the
> retired implementation did.  Read it for context; do not start new
> work against `TetradVM` — there is no `TetradVM` anymore.

## Overview

The Tetrad VM executes `CodeObject`s produced by the bytecode compiler (spec TET03).
It is a **register-based virtual machine** with an accumulator (following the V8 Ignition
model), an 8-register file, a software-managed call stack, and a built-in **metrics
layer** that records runtime observations into feedback vectors.

The metrics layer is the key contribution of this VM. Current Lisp implementations lack
a VM that cleanly exposes type feedback, branch statistics, and call-site shape in an
API that an external JIT can consume. The Tetrad VM provides this API from day one,
so the JIT compiler (spec TET05) — and any future Lisp front-end — can consume it
without modifying the interpreter core.

---

## Execution Model

```
CodeObject
    │
    ▼
┌──────────────────────────────────────────────────────┐
│                    VM State                          │
│                                                      │
│  acc           (current accumulator value: u8)       │
│  registers[8]  (R0–R7, each u8)                      │
│  ip            (instruction pointer: int)            │
│  call_stack    (list[CallFrame], max depth 4)        │
│  globals       (dict[str, u8])                       │
│  feedback_vectors  (dict[fn_name, list[SlotState]])  │
│  metrics       (VMMetrics, see below)                │
└──────────────────────────────────────────────────────┘
```

The VM is single-threaded. There is no concurrency model in v1.

---

## Call Frame

Each function call pushes a `CallFrame`. When the function returns, the frame is popped
and the accumulator value is passed back to the caller.

```python
@dataclass
class CallFrame:
    code: CodeObject               # function being executed
    ip: int                        # instruction pointer within this frame
    acc: int                       # accumulator value (u8, 0–255)
    registers: list[int]           # R0–R7 snapshot for this activation (8 ints)
    feedback_vector: list[SlotState]  # per-call feedback vector
    locals: dict[str, int]         # local variables (name → u8 value)
    caller_frame: CallFrame | None # back-pointer to caller
```

The `registers` array is per-frame, so callee functions cannot accidentally clobber the
caller's registers. On return, the caller's registers are restored from the saved frame.

The software call stack allows a maximum of 4 active frames (1 main + 3 nested calls).
This limit is enforced at runtime. Attempting a 5th level raises `VMError`.

This maps directly to the 4004 hardware model: the 4004's 3-level hardware call stack
supports at most 3 nested JMS instructions. The Tetrad VM adds one level for "main" and
uses software RAM to emulate the stack, giving 4 total levels.

---

## Feedback Vector

For **FULLY_TYPED** functions (`code.feedback_slot_count == 0`), no feedback vector is
allocated. The call frame's `feedback_vector` field is set to an empty list `[]` and
the VM never writes to it. This saves both RAM and one Python list allocation per call.

For all other functions, each `CallFrame` allocates a feedback vector of
`code.feedback_slot_count` slots, initialized to `:uninitialized`.

```python
class SlotKind(enum.Enum):
    UNINITIALIZED = "uninitialized"
    MONOMORPHIC   = "monomorphic"
    POLYMORPHIC   = "polymorphic"
    MEGAMORPHIC   = "megamorphic"

@dataclass
class SlotState:
    kind: SlotKind
    observations: list[str]  # type strings seen, e.g. ["u8"] or ["u8", "pair", "symbol"]
    count: int                # how many times this slot has been reached
```

For Tetrad v1, all values are `u8`, so every binary op slot stays `:monomorphic` with
`observations=["u8"]`. This data is still written — so the JIT can verify that it
reads the feedback correctly — it just never sees a polymorphic slot.

When a Lisp front-end is added, the slot for `(+ x y)` might see `["u8", "pair"]`
(if `+` is called with both numbers and pairs in the same loop), triggering the
polymorphic path and inhibiting the fast JIT specialization.

### Feedback Update Rules

```
uninitialized → monomorphic    (first observation, any type)
monomorphic   → monomorphic    (same type observed again)
monomorphic   → polymorphic    (new type seen; 2–4 types)
polymorphic   → polymorphic    (additional type, up to 4)
polymorphic   → megamorphic    (5th distinct type seen)
megamorphic   → megamorphic    (stays megamorphic forever)
```

Once megamorphic, the slot is never downgraded. This matches V8's behavior.

---

## Pre-execution: Immediate JIT Queue Population

Before the dispatch loop begins, the VM walks all top-level function `CodeObject`s and
populates the `immediate_jit_queue` with any that have `immediate_jit_eligible = True`:

```python
def execute(self, code: CodeObject) -> int:
    # Populate the immediate JIT queue before first instruction
    for fn in code.functions:
        if fn.immediate_jit_eligible:
            self.metrics.immediate_jit_queue.append(fn.name)
    # The JIT (if attached) drains this queue and compiles before the loop starts.
    # The interpreter proceeds regardless — compiled code is used on first call.
    ...
```

When the JIT is attached (via `TetradJIT.execute_with_jit`), it drains this queue and
compiles each FULLY_TYPED function **before the first instruction of main executes**.
This guarantees zero-warmup for typed functions: the very first call goes to native code.

## Dispatch Loop

The VM's inner loop is a fetch-decode-execute cycle:

```python
def run(frame: CallFrame) -> int:
    while True:
        instr = frame.code.instructions[frame.ip]
        opcode = instr.opcode
        frame.ip += 1

        # --- Accumulator loads ---
        if opcode == 0x00:   # LDA_IMM
            frame.acc = instr.operands[0]

        elif opcode == 0x01: # LDA_ZERO
            frame.acc = 0

        elif opcode == 0x02: # LDA_REG
            frame.acc = frame.registers[instr.operands[0]]

        elif opcode == 0x03: # LDA_VAR
            idx = instr.operands[0]
            name = frame.code.var_names[idx]
            frame.acc = frame.locals.get(name, globals.get(name, 0))

        # --- Stores ---
        elif opcode == 0x10: # STA_REG
            frame.registers[instr.operands[0]] = frame.acc

        elif opcode == 0x11: # STA_VAR
            idx = instr.operands[0]
            name = frame.code.var_names[idx]
            if name in frame.locals:
                frame.locals[name] = frame.acc
            else:
                globals[name] = frame.acc

        # --- Arithmetic ---
        elif opcode == 0x20: # ADD r, slot
            r, slot = instr.operands
            result = (frame.acc + frame.registers[r]) % 256
            record_binary_op(frame, slot, "u8", "u8")
            metrics.record_instruction(0x20)
            frame.acc = result

        # ... (all other opcodes follow same pattern)

        # --- Control flow ---
        elif opcode == 0x60: # JMP
            offset = signed16(instr.operands[0], instr.operands[1])
            frame.ip += offset

        elif opcode == 0x61: # JZ
            offset = signed16(instr.operands[0], instr.operands[1])
            slot_idx = instr.operands[2] if len(instr.operands) > 2 else None
            taken = (frame.acc == 0)
            if slot_idx is not None:
                record_branch(frame, slot_idx, taken)
            if taken:
                frame.ip += offset

        elif opcode == 0x63: # JMP_LOOP (backward jump)
            offset = signed16(instr.operands[0], instr.operands[1])
            metrics.record_loop_back_edge(frame.code.name, frame.ip)
            frame.ip += offset

        # --- Calls ---
        elif opcode == 0x70: # CALL
            func_idx, argc, slot = instr.operands
            callee = frame.code.functions[func_idx]
            record_call_site(frame, slot, callee.name)
            # ... push new frame, copy arg registers, run callee

        elif opcode == 0x71: # RET
            result = frame.acc
            if frame.caller_frame is None:
                return result   # top-level return → exit
            caller = frame.caller_frame
            caller.acc = result
            frame = caller      # pop frame

        # --- Halt ---
        elif opcode == 0xFF:
            return frame.acc
```

---

## Metrics Layer

The metrics layer is the distinguishing feature of the Tetrad VM. It is a first-class
subsystem, not an afterthought. Every meaningful runtime event is recorded.

### VMMetrics Structure

```python
@dataclass
class VMMetrics:
    # Per-instruction execution counts (opcode → count)
    instruction_counts: dict[int, int]

    # Per-function execution counts (function name → invocation count)
    function_call_counts: dict[str, int]

    # Per-loop back-edge counts (function name → list of (ip, count) pairs)
    # ip is the position of the JMP_LOOP instruction
    loop_back_edge_counts: dict[str, dict[int, int]]

    # Per-branch statistics (function name → slot → BranchStats)
    branch_stats: dict[str, dict[int, BranchStats]]

    # Total instructions executed
    total_instructions: int

    # Functions queued for immediate JIT compilation (FULLY_TYPED functions).
    # The JIT reads this queue and compiles these before they are first called.
    # This is the mechanism by which typed functions skip warmup entirely.
    immediate_jit_queue: list[str]   # function names, in declaration order

@dataclass
class BranchStats:
    taken_count: int
    not_taken_count: int

    @property
    def taken_ratio(self) -> float:
        total = self.taken_count + self.not_taken_count
        return self.taken_count / total if total > 0 else 0.0
```

### Metrics API (for JIT and external consumers)

```python
class TetradVM:

    # Execute a compiled CodeObject. Returns the final accumulator value.
    def execute(self, code: CodeObject) -> int: ...

    # Execute with full step-by-step trace (slow path, for debugging).
    def execute_traced(self, code: CodeObject) -> tuple[int, list[VMTrace]]: ...

    # Return hot functions: those called more than `threshold` times.
    def hot_functions(self, threshold: int = 100) -> list[str]: ...

    # Return the feedback vector for a named function after execution.
    # Indexed by feedback slot. Returns None if function not yet called.
    def feedback_vector(self, fn_name: str) -> list[SlotState] | None: ...

    # Return the type profile for one feedback slot in one function.
    # Returns the SlotState (kind + observations).
    def type_profile(self, fn_name: str, slot: int) -> SlotState | None: ...

    # Return branch statistics for one feedback slot (JZ/JNZ) in one function.
    def branch_profile(self, fn_name: str, slot: int) -> BranchStats | None: ...

    # Return loop iteration counts for all back-edges in a function.
    # dict maps instruction offset of JMP_LOOP → iteration count.
    def loop_iterations(self, fn_name: str) -> dict[int, int]: ...

    # Return call site shape for a slot (monomorphic/polymorphic/megamorphic).
    def call_site_shape(self, fn_name: str, slot: int) -> SlotKind: ...

    # Return the raw VMMetrics object (for external analysis tools).
    def metrics(self) -> VMMetrics: ...

    # Reset all metrics (useful for benchmarking steady state).
    def reset_metrics(self) -> None: ...
```

### Why These Metrics?

Each metric directly informs a JIT optimization decision:

| Metric | JIT decision it enables |
|---|---|
| `hot_functions` | Which functions to compile at all |
| `type_profile` | Type specialization (emit integer-specific add instead of generic dispatch) |
| `branch_profile` | Branch layout: put the hot path first, cold path last |
| `loop_iterations` | Loop unrolling threshold; OSR (on-stack replacement) trigger |
| `call_site_shape` | Inlining: monomorphic sites are safe to inline; megamorphic are not |

### Lisp-specific metrics (future)

When a Lisp front-end is added, the same API gains meaning that v1 doesn't exercise:

- `type_profile` will return polymorphic slots for `(car x)` called on both pairs and
  numbers — the JIT will generate a type guard + fast path.
- `call_site_shape` will become megamorphic for `(apply fn args)` — the JIT skips
  inlining and falls through to the dispatch table.
- `branch_profile` will show `(null? x)` at 99% taken — the JIT inverts the branch
  and makes the rare non-null case a deoptimization point.

This is the design intent: the metrics API is stable and rich enough to support both
the simple u8-typed Tetrad front-end and a future dynamically-typed Lisp front-end,
without any change to the VM.

---

## Execution Trace (debug path)

When `execute_traced` is called, the VM records a `VMTrace` for every instruction:

```python
@dataclass
class VMTrace:
    frame_depth: int           # call depth (0 = top level)
    fn_name: str               # function being executed
    ip: int                    # instruction pointer before this step
    instruction: Instruction   # the instruction executed
    acc_before: int            # accumulator value before
    acc_after: int             # accumulator value after
    registers_before: list[int]  # register file snapshot before
    registers_after: list[int]   # register file snapshot after
    feedback_delta: list[tuple[int, SlotState]]
    # list of (slot_index, new_state) pairs changed by this instruction
```

The trace is expensive (one object per instruction) and is intended only for
debuggers, unit tests, and the literate explanations in the spec.

---

## Error Handling

The VM raises `VMError` for:

| Condition | Message |
|---|---|
| Division by zero | `division by zero at fn 'name' ip N` |
| Call stack overflow | `call stack overflow: max depth 4 exceeded` |
| Unknown opcode | `unknown opcode 0xNN at fn 'name' ip N` |
| Undefined variable | `undefined variable 'name' at fn 'fn_name' ip N` |
| Undefined function | `undefined function index N at fn 'fn_name'` |
| Wrong argument count | `'name' expects M args, got N` |

---

## RAM Budget on Intel 4004

When the VM is implemented in 4004 assembly (the physical target), the VM state maps
to the 4004's RAM as follows:

```
Address  Size  Content
──────────────────────────────────────────────────────────
0x00     1     Accumulator (current u8 value)
0x01     8     Registers R0–R7 (one byte each)
0x09     2     IP (instruction pointer, 12-bit in 2 bytes)
0x0B     1     Current frame index (0–3)
0x0C     32    Call stack (4 frames × 8 bytes):
                 Frame N+0: return IP low byte
                 Frame N+1: return IP high byte
                 Frame N+2: saved register R0
                 Frame N+3: variable pool base (frame offset)
                 Frame N+4–7: reserved
0x2C     84    Variable pool (84 × 1-byte u8 variables)
──────────────────────────────────────────────────────────
Total:   128 bytes  (exactly the 4004's usable general RAM)
```

The feedback vector and metrics are **not** stored in 4004 RAM — there is no room.
They exist only in the Python VM running on a host. When running on physical 4004 silicon,
the metrics API is simply absent (the interpreter runs without instrumentation).

---

## Python Package

The VM lives in `code/packages/python/tetrad-vm/`.

Depends on `coding-adventures-tetrad-compiler`.

### Public API

```python
from tetrad_vm import TetradVM, VMError, VMTrace
from tetrad_vm.metrics import VMMetrics, SlotState, SlotKind, BranchStats

vm = TetradVM()
result = vm.execute(code_object)       # fast path
result, trace = vm.execute_traced(code_object)  # slow path with trace

# Metrics inspection
hot = vm.hot_functions(threshold=50)
profile = vm.type_profile("multiply", slot=0)
branches = vm.branch_profile("count_down", slot=0)
loops = vm.loop_iterations("count_down")
shape = vm.call_site_shape("main", slot=0)
```

---

## Test Strategy

### Opcode tests (unit)

Test each opcode in isolation: construct a `CodeObject` with one or two instructions,
execute, verify accumulator and register values.

- `LDA_IMM 42` → acc == 42
- `LDA_ZERO` → acc == 0
- `STA_REG 3` → R[3] == acc
- `ADD r=0, slot=0` with acc=10, R[0]=5 → acc == 15; slot 0 is monomorphic u8
- `SUB r=0, slot=0` with acc=5, R[0]=10 → acc == 251 (wraps at 256)
- `MUL r=0, slot=0` with acc=3, R[0]=4 → acc == 12
- `DIV r=0, slot=0` with R[0]=0 → VMError (division by zero)
- `AND r=0` with acc=0xFF, R[0]=0x0F → acc == 0x0F
- `NOT` with acc=0x0F → acc == 0xF0
- `JZ offset` with acc=0 → ip advances by offset
- `JZ offset` with acc=5 → ip does not jump
- `JMP_LOOP` → loop_back_edge count increments

### Feedback vector tests

- After `ADD r=0, slot=0`: `feedback_vector('f')[0].kind == MONOMORPHIC`
- After same slot observes same type twice: kind stays MONOMORPHIC, count == 2

### Metrics API tests

- `hot_functions(100)` returns empty list after <100 calls
- `hot_functions(100)` returns `['f']` after f called 101 times
- `loop_iterations('count_down')` counts backward jumps
- `branch_profile('count_down', slot=0)` tracks taken/not-taken correctly

### Call stack tests

- Nested call pushes new frame; registers are isolated between frames
- Return transfers acc value to caller's acc
- 5th call level raises `VMError` (stack overflow)

### End-to-end tests

Execute all five TET00 example programs, capture `IO_OUT` output, verify against
expected output sequence.

### Coverage target

95%+ line coverage.

---

## Version History

| Version | Date | Description |
|---|---|---|
| 0.1.0 | 2026-04-20 | Initial specification |
