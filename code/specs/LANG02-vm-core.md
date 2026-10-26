# LANG02 — vm-core: Generic Register VM Interpreter

## Overview

`vm-core` is a **language-agnostic register virtual machine** that executes
`IIRModule` programs (LANG01).  It replaces `TetradVM`, `LogicVM`, and every
other per-language interpreter in this repository with a single, tested,
optimised dispatch loop.

The key insight from building `TetradVM` is that the interesting part of a
register VM is not the opcodes — it is the **dispatch loop** and the
**profiler**.  The opcodes change per language (add, mul, string-concat, …);
the dispatch infrastructure does not.  `vm-core` owns the dispatch infrastructure
and receives the opcode set as a plug-in.

This spec covers:

1. The dispatch loop and register file
2. The call-frame stack
3. The profiler (feedback vectors)
4. The builtins registry
5. The metrics API consumed by `jit-core`
6. Configuration and language-specific plug-ins

---

## Architecture

```
IIRModule
   │
   ▼
VMCore.execute(module)
   │
   ├── Phase 1: compile FULLY_TYPED functions to InterpreterIR
   │     (no-op for vm-core — compilation happens in jit-core)
   │
   ├── Phase 2: push entry-point frame onto call stack
   │
   └── dispatch_loop()
         │
         ├── fetch IIRInstr at frame.ip
         ├── decode op → handler
         ├── execute handler (mutates register file, pushes/pops frames)
         ├── if profiling enabled: profile_instr(instr, result)
         └── advance frame.ip
```

### Relationship to TetradVM

`TetradVM` was a 350-line Python class implementing:

- A 8-register file
- A 4-frame call stack
- An opcode dispatch table for ~20 Tetrad-specific opcodes
- A metrics counter for function call counts

`vm-core` generalises all four of those components.  After migration:

```
TetradVM                     →  VMCore(register_count=8, max_frames=4,
                                        opcodes=tetrad_opcode_table)
tetrad_vm.metrics()          →  vm_core.metrics()     (same structure)
tetrad_vm.execute(code)      →  vm_core.execute(module)
```

---

## Register file

```python
class RegisterFile:
    """A flat array of 'any'-typed slots.

    Languages with known types (Tetrad: u8) benefit from narrowing the slots
    to int at construction time; dynamically typed languages leave them as Any.
    """

    def __init__(self, count: int = 8, dtype: type = int) -> None:
        self._regs: list[Any] = [dtype(0)] * count

    def __getitem__(self, idx: int) -> Any:
        return self._regs[idx]

    def __setitem__(self, idx: int, value: Any) -> None:
        self._regs[idx] = value

    def reset(self) -> None:
        for i in range(len(self._regs)):
            self._regs[i] = 0
```

---

## Call frame

```python
@dataclass
class VMFrame:
    fn: IIRFunction          # the function being executed
    ip: int = 0              # instruction pointer (index into fn.instructions)
    registers: RegisterFile  # per-frame register file
    return_dest: int | None  # register index to store return value in caller
```

The call stack is a bounded deque.  The maximum frame depth is configurable
(default: `max_frames=64`, matching JVM defaults; Tetrad uses `max_frames=4`).

### Frame register limit

A frame holds at most **65,536 (2¹⁶) distinct named registers**
(`VMFrame::MAX_FRAME_REGISTERS` in the Rust implementation). This is a
resource bound — it caps one frame's register file — and the same 2¹⁶ bounds
the `load_reg`/`store_reg` index range (`0..=65_535`). What the whole call
stack may hold is bounded separately, by the register-slot budget below.

A function's registers are its **parameters, by position** — parameter `i`
owns slot `i` even when two parameters share a name — plus the distinct
`dest` names of its instructions that are not parameter names (a `dest`
naming a parameter reuses its slot). The count is static, and new registers
are numbered from `params.len()`. Exceeding the limit is a typed error,
never a silent drop or a panic:

1. **Load time.** Before running anything, `execute` counts the distinct
   register names of every function in the module. A function over the limit
   refuses the whole module with `RegisterLimitExceeded { function, registers,
   limit }` — no instruction runs and no output is written.
2. **Run time (backstop).** Allocating a new register in a full frame raises
   the same error, for a register a language opcode handler creates
   dynamically.

`register_count` is a pre-allocation hint only. The VM pre-allocates it
bounded by what the function can ever name (`params + instructions`) and by
the limit, and never below `params.len()`.

**Register-slot budget (memory bound).** The per-frame limit caps one
frame, not one program: `max_frames` (default 512 in the Rust vm-core) ×
65,536 slots × ~24 bytes is ~768 MiB, and a three-instruction recursive
function reaches it — `store_reg 65535, 1; call r = f; ret r` grows every
frame to the limit with one by-index write (measured peak RSS ≈ 792 MB before
`FrameOverflow`). So all live frames of one call stack also share a
**VM-wide budget of register slots**, `max_register_slots` (default
2²² = 4,194,304):

- **What is counted:** each frame's register-file *capacity* (allocated
  slots, not just used ones), summed over every live frame.
- **When it is checked:** whenever a register file grows — a new named
  register (including `call`'s return slot), a `store_reg` past the end of
  the file, or a `call` pre-allocating a callee frame (at least
  `params.len()` slots). Growth over the budget is the typed error
  `RegisterBudgetExceeded { function, slots, budget }` (its message names
  the knob: "raise VMCore::max_register_slots"); the frame is left
  unchanged. A failed `execute` clears its call stack before returning, so
  the frames' register files are released at once rather than at the next
  `execute`.
- **How it is counted (O(1), no pop bookkeeping):** each frame records,
  when it is pushed, the slots held by the frames beneath it
  (`slots_below`). Only the top frame ever grows — a caller cannot run while
  its callee does, and `call` allocates the caller's return slot *before*
  pushing the callee — so that number stays exact, and a frame may grow
  while `slots_below + capacity ≤ budget`. Popping a frame takes its share
  with it. Growth doubles capacity (amortised O(1)) but is clamped to what
  the budget leaves, so capacity never overshoots.
- **The bound:** the register files of one `execute` occupy at most
  `max_register_slots × size_of::<Value>()` bytes — **96 MiB** at the
  default, 8× below the ~768 MiB it prevents.
- **What the default guarantees.** Capacity runs ahead of use: growth
  doubles from the pre-allocation (the `register_count` hint, 8 for almost
  every frontend), so a frame using `u` slots holds at most
  `max(pre-allocation, 2u − 1)` — the doubling slack is under 2×. The
  default gives the full 512-frame stack 8,192 slots a frame, so a function
  whose frames use ≤ 4,096 registers (and pre-allocate ≤ 8,192) still
  reaches `max_frames` = 512 and ends in `FrameOverflow` exactly as before
  the budget, and frames of up to the 65,536-register limit reach depth 64.
  (A 2²⁰ default was tried first: with the slack, a 2,049-register
  recursive function stopped at depth 255 instead of 512.)
- **What it does not change:** the `load_reg`/`store_reg` index range stays
  `0..=65_535` (a negative or larger index is still an
  `invalid register index` error). No frontend in the repository emits
  numeric-index `load_reg`/`store_reg` for vm-core — `twig-to-beam` emits
  `load_reg` as a BEAM register copy, and the BEAM/CIL/JVM backends lower
  both to moves — so tying the index range to the function's static
  register count was rejected as a semantic change with no caller to
  justify it; the budget bounds memory without it.

Not counted: the name map's key strings (one copy per frame of each `dest`
name the frame assigns — at most one entry per slot, so their *number* is
bounded by the budget, but a pathologically long `dest` name is copied into
every frame that uses it) and the contents of `Value::Str` registers.
Host opcode extensions that grow `registers` directly (it is a `pub` field)
are trusted and outside the accounting; they should grow it through
`VMFrame::store_index` / `assign`.

> History: the Rust vm-core originally dropped the 65,537th register
> silently, so the program failed later with a misleading
> `undefined variable`. The JIT tier (`jit-core`'s generic bytecode JIT) has
> a smaller, separate limit — 256 registers, one-byte register numbers — and
> declines to compile such a function, leaving it to the interpreter.

When a `call` instruction is executed:

1. A new `VMFrame` is allocated for the callee
2. Arguments are copied from the caller's register file to the callee's registers
3. The callee frame is pushed onto the call stack
4. `dispatch_loop` continues with the callee's first instruction

When a `ret` instruction is executed:

1. The return value is placed in `frame.return_dest` of the *caller* frame
2. The callee frame is popped
3. The caller's `ip` is advanced past the `call` instruction

---

## Dispatch loop

The dispatch loop is a tight `while True` cycle:

```python
def dispatch_loop(self) -> int | None:
    while self._frames:
        frame = self._frames[-1]          # current frame (top of stack)
        if frame.ip >= len(frame.fn.instructions):
            # Fell off the end of the function without a ret instruction.
            # Treat as ret None.
            self._frames.pop()
            continue

        instr = frame.fn.instructions[frame.ip]
        result = self._dispatch(frame, instr)

        if self._profiler_enabled:
            self._profiler.observe(instr, result)

        frame.ip += 1

    return self._return_value
```

`_dispatch` is a dictionary from opcode string to handler callable:

```python
def _dispatch(self, frame: VMFrame, instr: IIRInstr) -> Any:
    handler = self._opcode_table.get(instr.op)
    if handler is None:
        raise VMError(f"unknown opcode: {instr.op!r}")
    return handler(self, frame, instr)
```

Each handler is a function:

```python
def handle_add(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = resolve(frame, instr.srcs[0])
    b = resolve(frame, instr.srcs[1])
    result = (a + b) & 0xFF   # u8 wrap for Tetrad; language-specific
    frame.registers[dest_idx(instr.dest)] = result
    return result
```

`resolve(frame, src)` returns `src` directly if it is a literal, or looks up
`frame.registers[name_to_idx[src]]` if it is a string variable name.

### Opcode table construction

Each language supplies its opcode table at VM construction time:

```python
from vm_core import VMCore
from tetrad_opcode_table import TETRAD_OPCODES

vm = VMCore(
    register_count=8,
    max_frames=4,
    opcodes=TETRAD_OPCODES,
)
```

`TETRAD_OPCODES` is a `dict[str, Callable]` mapping mnemonic strings to
handler functions.  The generic opcodes (`const`, `jmp`, `label`, `ret`,
`load_reg`, `store_reg`, `call`) are provided by `vm-core` itself and do not
need to be specified by the language.

---

## Profiler

The profiler is an optional component that observes instruction results and
fills in the `observed_type` and `observation_count` fields of `IIRInstr`.

```python
class VMProfiler:
    def observe(self, instr: IIRInstr, result: Any) -> None:
        if instr.type_hint != "any":
            # Already statically typed — nothing to observe.
            return
        rt = _python_type_to_iir(type(result))
        if instr.observed_type is None:
            instr.observed_type = rt
        elif instr.observed_type != rt:
            instr.observed_type = "polymorphic"
        instr.observation_count += 1
```

`_python_type_to_iir` maps Python types to IIR type strings:
- `int` in range 0–255 → `"u8"`
- `int` in range 0–65535 → `"u16"`
- `int` larger → `"u32"` or `"u64"`
- `bool` → `"bool"`
- `str` → `"str"`
- anything else → `"any"`

The profiler overhead is constant per instruction and is always on.  It
does not affect correctness — it only annotates the `IIRInstr` objects in
place.

---

## Metrics API

`vm-core` exposes a metrics snapshot consumed by `jit-core`:

```python
@dataclass
class VMMetrics:
    function_call_counts: dict[str, int]   # fn_name → call count
    total_instructions_executed: int
    total_frames_pushed: int
    total_jit_hits: int                    # calls that bypassed interpreter
```

```python
vm.metrics() -> VMMetrics
```

This is the same interface as `TetradVM.metrics()`.  `jit-core` reads
`function_call_counts` to decide when a function crosses its promotion
threshold.

---

## Builtins registry

Languages need host-provided operations (I/O, string formatting, …) that
cannot be expressed as `IIRInstr` sequences.  These are registered as
**builtins**:

```python
vm.register_builtin("print", lambda args: print(args[0]))
vm.register_builtin("input", lambda args: input())
```

`call_builtin` instructions in `IIRFunction` are dispatched to the registered
Python callable.  This is the seam through which BASIC's `PRINT`, Lua's
`print()`, and Python's `print()` are all handled without any VM modification.

---

## Shadow frames (for JIT deopt)

When `jit-core` compiles a function, the interpreter's frame for that function
becomes a **shadow frame** — it is suspended but kept alive so that a deopt can
resume from it.

`vm-core` exposes:

```python
vm.suspend_frame(fn_name: str) -> VMFrame   # pause interpreter; hand frame to JIT
vm.resume_frame(frame: VMFrame, at_ip: int) # restart interpreter from deopt point
```

`jit-core` calls `suspend_frame` when entering a compiled function and
`resume_frame` when a type guard fails.

---

## Configuration

```python
@dataclass
class VMConfig:
    register_count: int = 8          # registers per frame
    max_frames: int = 64             # maximum call depth
    max_register_slots: int = 2**20  # register slots all live frames share
                                     # (Rust vm-core; see "Frame register limit")
    opcodes: dict = field(default_factory=dict)   # language-specific handlers
    builtins: dict = field(default_factory=dict)  # host-provided callables
    profiler_enabled: bool = True    # always-on observation
    u8_wrap: bool = False            # wrap arithmetic results to 0–255 (Tetrad)
```

`u8_wrap=True` applies `& 0xFF` to every arithmetic result — used by Tetrad to
match its u8 semantics without putting the mask in every opcode handler.

---

## Package structure

```
vm-core/
  pyproject.toml
  BUILD
  README.md
  CHANGELOG.md
  src/vm_core/
    __init__.py       # exports VMCore, VMConfig, VMMetrics
    core.py           # VMCore class — top-level API
    frame.py          # VMFrame, RegisterFile
    dispatch.py       # dispatch_loop, _dispatch, opcode table resolution
    profiler.py       # VMProfiler — type observation
    builtins.py       # built-in registry + standard builtins (no-op, io_in, io_out)
    metrics.py        # VMMetrics dataclass
    shadow.py         # shadow frame suspend / resume for JIT deopt
    errors.py         # VMError, DeoptimizerError, FrameOverflowError
  tests/
    test_arithmetic.py
    test_control_flow.py
    test_call_frames.py
    test_profiler.py
    test_builtins.py
    test_shadow_frames.py
```

---

## What vm-core does NOT do

- **Compile code** — `vm-core` executes `IIRModule`; compilation is the
  frontend's job
- **Generate native binaries** — that is `jit-core` + backend
- **Manage garbage collection** — values live in register slots; GC is a
  future `gc-core` concern (GC00+)
- **Parse source** — lexer + parser are language-specific frontends

---

## Migration path

### TetradVM → VMCore

```python
# Before
from tetrad_vm import TetradVM
vm = TetradVM()
result = vm.execute(code_object)

# After
from vm_core import VMCore
from tetrad_opcodes import TETRAD_OPCODES   # thin adapter over existing Op handlers
vm = VMCore(register_count=8, max_frames=4, opcodes=TETRAD_OPCODES, u8_wrap=True)
result = vm.execute(iir_module)
```

The `tetrad-vm` package retains its public API as a compatibility shim that
constructs a `VMCore` with Tetrad defaults.  No existing Tetrad tests break.
