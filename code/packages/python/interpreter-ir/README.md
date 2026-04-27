# interpreter-ir

**InterpreterIR** — the shared dynamic bytecode IR for the generic language
pipeline (spec LANG01).

Any language whose compiler emits `IIRModule` automatically gains a generic
VM (`vm-core`), JIT compiler (`jit-core`), VSCode debugger, language server,
interactive REPL, and Jupyter notebook kernel — for free.

## What it is

`interpreter-ir` defines three core types:

| Type | Role |
|------|------|
| `IIRInstr` | A single instruction with op, dest, srcs, type hint, and profiler feedback slots |
| `IIRFunction` | A named, parameterised sequence of `IIRInstr` |
| `IIRModule` | Top-level container: all functions + entry point |

## Quick start

```python
from interpreter_ir import IIRInstr, IIRFunction, IIRModule, FunctionTypeStatus
from interpreter_ir import serialise, deserialise

instrs = [
    IIRInstr("add", "v0", ["a", "b"], "u8"),
    IIRInstr("ret",  None, ["v0"],    "u8"),
]
fn = IIRFunction(
    name="add",
    params=[("a", "u8"), ("b", "u8")],
    return_type="u8",
    instructions=instrs,
    type_status=FunctionTypeStatus.FULLY_TYPED,
)
module = IIRModule(name="example", functions=[fn], entry_point="add")

# Serialise to bytes (for AOT snapshots or disk caching)
raw = serialise(module)
restored = deserialise(raw)
assert restored == module
```

## Type observation

For dynamically typed languages, the VM profiler fills in `observed_type`:

```python
instr = IIRInstr("add", "v0", ["a", "b"], "any")
# After vm-core executes this instruction:
instr.record_observation("u8")   # first call: u8
instr.record_observation("u8")   # second call: still u8
assert instr.observed_type == "u8"
assert instr.observation_count == 2

instr.record_observation("str")  # different type!
assert instr.observed_type == "polymorphic"  # don't specialise
```

## Opcode sets

```python
from interpreter_ir import ARITHMETIC_OPS, CMP_OPS, SIDE_EFFECT_OPS

if instr.op in ARITHMETIC_OPS:
    ...  # add, sub, mul, div, mod, neg
```

## Pipeline position

```
Language source
  → lexer → parser → type-checker
  → bytecode compiler  →  IIRModule   ← YOU ARE HERE
  → vm-core (interprets)
  → jit-core (compiles hot functions to CompilerIR → native binary)
```

## Dependencies

None. `interpreter-ir` is a zero-dependency leaf package.
