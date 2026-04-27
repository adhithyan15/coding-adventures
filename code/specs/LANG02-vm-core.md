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
