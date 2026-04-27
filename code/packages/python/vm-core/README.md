# vm-core

Generic register VM interpreter for the LANG pipeline (LANG02).

`vm-core` executes **InterpreterIR** (`interpreter-ir`) modules — the dynamic,
feedback-annotated IR produced by any language front-end in this pipeline.  It
is the shared, language-agnostic execution engine that sits between the parser
and the JIT compiler (`jit-core`, LANG03).

## Where it fits

```
language source
      │
      ▼
lexer → parser → type-checker
      │
      ▼
  IIRModule  ◀─── interpreter-ir (LANG01)
      │
      ▼
   VMCore    ◀─── vm-core (LANG02)  ← you are here
      │
      ├─ type feedback ──▶ jit-core (LANG03)
      └─ metrics       ──▶ jit-core
```

## Key concepts

### Register file

Each function call gets a flat `RegisterFile` — a list of value slots.
Parameters occupy slots 0..N-1; temporaries are allocated on demand.
Named variables are resolved through a `name_to_reg` dict on `VMFrame`.

### Dispatch loop

The loop is a tight `while frames:` cycle:

1. Fetch the current instruction (`frame.fn.instructions[frame.ip]`)
2. Advance `frame.ip`
3. Look up the handler in the opcode table (O(1) dict)
4. Call the handler — reads registers, writes result
5. If profiling is enabled, observe the result's runtime type

### JIT integration

Call `vm.register_jit_handler(fn_name, handler)` to bypass the interpreter
for a compiled function.  The handler receives a list of resolved argument
values and returns the result.  No frame is pushed; the interpreter path is
skipped entirely.

### Builtin functions

Host-provided Python callables are registered via `vm.register_builtin()` or
through the `BuiltinRegistry` directly.  `noop` and `assert_eq` are
pre-registered.  The `call_builtin` instruction dispatches to these.

### u8_wrap mode

Pass `u8_wrap=True` to mask every arithmetic result with `& 0xFF`.  Required
for Tetrad compatibility (8-bit register semantics).

## Installation

```bash
pip install coding-adventures-vm-core
```

## Quick start

```python
from interpreter_ir import IIRFunction, IIRInstr, IIRModule
from vm_core import VMCore

# Build a trivial function: return 2 + 3
fn = IIRFunction(
    name="main",
    params=[],
    instructions=[
        IIRInstr(op="const", dest="a", srcs=[2]),
        IIRInstr(op="const", dest="b", srcs=[3]),
        IIRInstr(op="add",   dest="r", srcs=["a", "b"]),
        IIRInstr(op="ret",            srcs=["r"]),
    ],
    register_count=8,
)
module = IIRModule(functions={"main": fn})

vm = VMCore()
result = vm.execute(module)  # → 5
print(result)

# Inspect execution metrics.
m = vm.metrics()
print(m.total_instructions_executed)
print(m.function_call_counts)
```

## API

### `VMCore`

```python
VMCore(
    *,
    max_frames: int = 64,
    opcodes: dict | None = None,   # override / extend the standard opcode table
    builtins: BuiltinRegistry | None = None,
    profiler_enabled: bool = True,
    u8_wrap: bool = False,
)
```

| Method | Description |
|---|---|
| `execute(module, *, fn="main", args=None)` | Run a function; return its result |
| `metrics()` | Snapshot of execution statistics |
| `register_builtin(name, fn)` | Add a host callable |
| `register_jit_handler(fn_name, handler)` | Add a JIT shortcut |
| `unregister_jit_handler(fn_name)` | Remove a JIT shortcut |
| `interrupt()` | Signal the dispatch loop to stop |
| `reset()` | Clear execution state (keep metrics) |

### `VMMetrics`

| Field | Type | Description |
|---|---|---|
| `function_call_counts` | `dict[str, int]` | Interpreted call count per function |
| `total_instructions_executed` | `int` | Cumulative instruction dispatch count |
| `total_frames_pushed` | `int` | Cumulative call frame count |
| `total_jit_hits` | `int` | Calls intercepted by JIT handlers |

## Running tests

```bash
uv venv
uv pip install -e ../interpreter-ir -e ".[dev]"
pytest tests/ -v
```
