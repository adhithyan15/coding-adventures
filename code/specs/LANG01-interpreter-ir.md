# LANG01 — InterpreterIR: Dynamic Bytecode Intermediate Representation

## Overview

`interpreter-ir` defines the **shared bytecode format** that all interpreted
languages in this repository compile to.  Before this package, each language
had its own bytecode (`CodeObject`/`Op` for Tetrad, `LogicInstruction` for the
logic engine, etc.).  After adoption, every language's bytecode compiler emits
`InterpreterIR` and every language's VM is replaced by the generic `vm-core`
(LANG02).

This spec covers:

1. The `IIRInstr` instruction dataclass
2. The full instruction set
3. Type observation slots (how the VM feeds back runtime types to the JIT)
4. Deoptimisation anchors
5. The `IIRFunction` and `IIRModule` container types
6. Serialisation format

---

## Design goals

| Goal | Rationale |
|------|-----------|
| **Interpreter-friendly** | Instructions are simple enough for a dispatch loop to execute without additional lowering |
| **JIT-transparent** | Feedback slots are embedded in the IR so the JIT can read observations without a separate data structure |
| **Language-agnostic** | No Tetrad-specific types; any language that compiles to u8/u16/u32/u64/bool/str values can use the same instruction set |
| **Deopt-safe** | Every compiled frame can fall back to the interpreter at a marked anchor point |
| **Serialisable** | `IIRModule` can be written to disk and read back, enabling AOT snapshot builds |

---

## Core dataclass: `IIRInstr`

```python
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any

@dataclass
class IIRInstr:
    op: str              # instruction mnemonic (see table below)
    dest: str | None     # destination variable name (SSA); None for void ops
    srcs: list[str | int | float | bool]  # source operands (names or literals)
    type_hint: str       # declared type: "u8", "u16", "u32", "u64", "bool", "str", "any"

    # Runtime feedback (written by vm-core profiler, read by jit-core)
    observed_type: str | None = field(default=None, repr=False)
    observation_count: int    = field(default=0,    repr=False)

    # Deopt anchor: if not None, jit-core must emit a guard that reverts to
    # interpreter execution at this instruction index when the guard fails.
    deopt_anchor: int | None  = field(default=None, repr=False)
```

### Field semantics

**`op`** — the instruction mnemonic.  All mnemonics are lowercase strings
(e.g., `"add"`, `"cmp_lt"`, `"jmp_if_false"`, `"call"`).

**`dest`** — the name of the SSA variable produced by this instruction, or
`None` for instructions that produce no value (returns, stores, branches).

**`srcs`** — a list of operands.  Each element is either:
- A `str` naming another SSA variable defined earlier in the same function
- An `int`, `float`, or `bool` literal (immediate values)

**`type_hint`** — the declared type.  For statically typed languages (Tetrad),
this is filled in by the type-checker.  For dynamically typed languages
(Lua, BASIC), this is `"any"` until the profiler fills in `observed_type`.

**`observed_type`** — set by the `vm-core` profiler after seeing the actual
runtime type of `dest`.  Initially `None`.  When filled in for a
`type_hint="any"` instruction, the JIT can specialize to that type.

**`observation_count`** — how many times this instruction has been profiled.
Used by `jit-core` to decide whether it has enough observations to specialize.

**`deopt_anchor`** — when the JIT specializes on `observed_type`, it emits a
type guard before the specialized path.  If the guard fails at runtime, the
compiled frame is abandoned and execution restarts at the interpreter from
instruction index `deopt_anchor`.

---

## Instruction set

### Arithmetic

| Mnemonic | Operands | Description |
|----------|----------|-------------|
| `const`  | (literal,) | Load an immediate value into `dest` |
| `add`    | (a, b) | `dest = a + b` |
| `sub`    | (a, b) | `dest = a - b` |
| `mul`    | (a, b) | `dest = a * b` |
| `div`    | (a, b) | `dest = a / b` (integer division) |
| `mod`    | (a, b) | `dest = a % b` |
| `neg`    | (a,) | `dest = -a` |

### Bitwise

| Mnemonic | Operands | Description |
|----------|----------|-------------|
| `and`    | (a, b) | `dest = a & b` |
| `or`     | (a, b) | `dest = a \| b` |
| `xor`    | (a, b) | `dest = a ^ b` |
| `not`    | (a,) | `dest = ~a` |
| `shl`    | (a, n) | `dest = a << n` |
| `shr`    | (a, n) | `dest = a >> n` |

### Comparison

| Mnemonic | Operands | Description |
|----------|----------|-------------|
| `cmp_eq` | (a, b) | `dest = a == b` |
| `cmp_ne` | (a, b) | `dest = a != b` |
| `cmp_lt` | (a, b) | `dest = a < b` |
| `cmp_le` | (a, b) | `dest = a <= b` |
| `cmp_gt` | (a, b) | `dest = a > b` |
| `cmp_ge` | (a, b) | `dest = a >= b` |

### Control flow

| Mnemonic | `dest` | Operands | Description |
|----------|--------|----------|-------------|
| `jmp`      | None | (label,) | Unconditional jump to label |
| `jmp_if_true`  | None | (cond, label) | Jump if cond is truthy |
| `jmp_if_false` | None | (cond, label) | Jump if cond is falsy |
| `label`    | None | (name,) | Define a branch target |
| `ret`      | None | (value,) | Return value from function; value may be a variable name or literal |
| `ret_void` | None | () | Return from a void function |

### Memory / registers

| Mnemonic | Operands | Description |
|----------|----------|-------------|
| `load_reg`  | (register_index,) | Load from a VM register into `dest` |
| `store_reg` | (register_index, value) | Store a value into a VM register |
| `load_mem`  | (address,) | Load from RAM address into `dest` |
| `store_mem` | (address, value) | Store a value to a RAM address |

### Function calls

| Mnemonic | Operands | Description |
|----------|----------|-------------|
| `call`    | (fn_name_or_index, arg0, arg1, …) | Call a function; result in `dest` |
| `call_builtin` | (builtin_name, arg0, …) | Call a host-provided built-in |

### I/O

| Mnemonic | Operands | Description |
|----------|----------|-------------|
| `io_in`   | (port,) | Read from I/O port into `dest` |
| `io_out`  | (port, value) | Write value to I/O port |

### Type coercions

| Mnemonic | Operands | Description |
|----------|----------|-------------|
| `cast`    | (value, target_type) | Coerce value to target_type |
| `type_assert` | (value, expected_type) | Guard: deopt if value is not expected_type |

---

## Type observation protocol

When `vm-core` executes an `IIRInstr` whose `type_hint` is `"any"`, it records
the runtime type of the produced value:

```
profile_instr(instr, result_value):
    rt = runtime_type_of(result_value)   # "u8", "u16", "bool", "str", …
    if instr.observed_type is None:
        instr.observed_type = rt
    elif instr.observed_type != rt:
        instr.observed_type = "polymorphic"  # many types seen → don't specialize
    instr.observation_count += 1
```

The profiler runs inline in the interpreter loop.  The overhead is one string
comparison and one integer increment per profiled instruction — small enough to
keep always-on in the base interpreter.

The JIT reads `observed_type` during specialization.  Instructions where
`observed_type == "polymorphic"` are **not** specialized; they emit a generic
call to a runtime helper instead.

### Specialization threshold

The JIT-core (LANG03) specializes a function when:

1. Every instruction in the function body has `observation_count >= min_observations`
   (default: 5), **or**
2. The function's `IIRFunction.type_status` is `FULLY_TYPED` (all `type_hint`s
   are concrete, not `"any"`).

---

## Deoptimisation anchors

When the JIT specializes on `observed_type`, it must be able to fall back to
the interpreter if the guard fails at runtime.  The `deopt_anchor` field stores
the interpreter instruction index to resume at.

Example: a function that adds two values observed as `u8`:

```
IIRInstr(op="add", dest="v0", srcs=["a", "b"], type_hint="any",
         observed_type="u8", deopt_anchor=2)
```

The JIT emits approximately:

```
; Guard: are both a and b u8?
TEST_TYPE a, u8   ; jump to deopt_stub if not
TEST_TYPE b, u8
; Specialized path: 8-bit add
FIM P1, a_hi:a_lo
FIM P2, b_hi:b_lo
… 4004 add sequence …
JMP compiled_exit
; Deopt stub: restart interpreter from instruction 2
deopt_stub:
  RESTORE_INTERPRETER_STATE(frame, instr_idx=2)
  JUMP interpreter_loop
```

The interpreter state needed for deopt is maintained by `vm-core` in a
**shadow frame**: a small structure that tracks the current register values
and instruction pointer in a format the interpreter can resume from.

---

## Container types

### `IIRFunction`

```python
@dataclass
class IIRFunction:
    name: str
    params: list[tuple[str, str]]      # [(param_name, type_hint), …]
    return_type: str                   # "u8", "void", "any", …
    instructions: list[IIRInstr]
    register_count: int = 8            # number of VM registers used
    type_status: FunctionTypeStatus = FunctionTypeStatus.UNTYPED
    call_count: int = 0                # updated by vm-core profiler
```

### `IIRModule`

```python
@dataclass
class IIRModule:
    name: str                          # typically the source file name
    functions: list[IIRFunction]
    entry_point: str = "main"          # name of the entry function
    language: str = "unknown"          # "tetrad", "basic", "lua", …
```

---

## Serialisation

`IIRModule` can be serialised to a compact binary format for AOT snapshot
builds.  The format is a simple length-prefixed encoding:

```
Header:
  4 bytes  magic number  0x49 0x49 0x52 0x00  ("IIR\0")
  2 bytes  version       0x01 0x00
  4 bytes  function_count (little-endian u32)

For each IIRFunction:
  2 bytes  name_length
  N bytes  name (UTF-8)
  2 bytes  param_count
  For each param:
    2 bytes  name_length, N bytes name
    2 bytes  type_length, N bytes type
  2 bytes  return_type_length, N bytes return_type
  4 bytes  instruction_count
  For each IIRInstr:
    2 bytes  op_length, N bytes op
    1 byte   has_dest (0 or 1)
    2 bytes  dest_length, N bytes dest (if has_dest)
    1 byte   src_count
    For each src:
      1 byte   kind (0=str, 1=int, 2=float, 3=bool)
      (variable-length encoding of the value)
    2 bytes  type_hint_length, N bytes type_hint
```

The observation fields (`observed_type`, `observation_count`, `deopt_anchor`)
are **not** serialised — they are runtime state that accumulates fresh on each
run.

---

## Relationship to existing IRs

| IR | Package | Layer | Use |
|----|---------|-------|-----|
| `InterpreterIR` | `interpreter-ir` (new) | dynamic | bytecode that the VM executes |
| `CompilerIR` | `compiler-ir` | static SSA | input to optimizers and backends |
| `SemanticIR` | `semantic-ir` | high-level | typed IR above CompilerIR; used by compiled-language path |

The JIT-core (LANG03) is the only consumer that reads `InterpreterIR` and
writes `CompilerIR`.  All other consumers (vm-core, backends) see only one IR.

---

## Package structure

```
interpreter-ir/
  pyproject.toml
  BUILD
  README.md
  CHANGELOG.md
  src/interpreter_ir/
    __init__.py        # exports IIRInstr, IIRFunction, IIRModule, FunctionTypeStatus
    instr.py           # IIRInstr dataclass + type_hint constants
    function.py        # IIRFunction dataclass
    module.py          # IIRModule dataclass
    opcodes.py         # frozenset constants: ARITHMETIC_OPS, CMP_OPS, etc.
    serialise.py       # IIRModule → bytes / bytes → IIRModule
  tests/
    test_instr.py
    test_serialise.py
```

---

## Migration path for Tetrad

The Tetrad pipeline migrates in three steps:

1. **`tetrad-compiler`** emits `IIRModule` instead of `CodeObject`.
   The mapping is straightforward: `CodeObject.instructions` → `IIRFunction.instructions`,
   with each `Op` mapped to the corresponding `IIRInstr` mnemonic.

2. **`tetrad-vm`** is replaced by `vm-core` configured for Tetrad's register
   count and type status thresholds.  Existing tests verify identical results.

3. **`tetrad-jit`** is replaced by `jit-core` with the `intel4004-backend`
   selected.  Liveness-based register recycling and nibble-pair arithmetic
   move into the backend.
