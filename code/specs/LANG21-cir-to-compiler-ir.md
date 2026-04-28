# LANG21 — `cir-to-compiler-ir`: CIR-to-IrProgram Lowering Bridge

## Overview

LANG19 built `jit-core`, which specialises hot `IIRFunction`s into `list[CIRInstr]` — a
typed, SSA-shaped intermediate form produced by the JIT/AOT specialisation pass.

LANG20 gave every `ir-to-*` backend a `CodeGenerator[IrProgram, Assembly]` adapter —
a uniform interface for validating and generating target code from the architecture-
independent `IrProgram`.

The missing link is a **lowering pass** that converts `list[CIRInstr]` → `IrProgram`.
That is what LANG21 ships: the `cir-to-compiler-ir` package.

### Full pipeline once LANG21 ships

```
Tetrad source
  → tetrad-lexer / parser / type-checker / compiler → CodeObject
  → tetrad-runtime.compile_to_iir()                 → IIRModule
  → jit-core.specialise(fn, min_observations=0)     → list[CIRInstr]  (forced AOT)
  → CIROptimizer.run()                              → list[CIRInstr]
  → lower_cir_to_ir_program()       ← LANG21 (this package)
  → IrProgram
  → WASMCodeGenerator / JVMCodeGenerator / etc.     ← LANG20 adapters
  → WasmModule / JVMClassArtifact / …
  → wasm-runtime / GraalVM / GE-225 simulator / CIL simulator
```

With LANG21 in place, a Tetrad program can be compiled end-to-end through any of the
six existing backends (WASM, JVM, CIL, GE-225, Intel 4004, Intel 8008) and executed
on their simulators — without writing a single backend-specific code generator for
Tetrad.

---

## Why a separate lowering package?

`CIRInstr` is a typed, SSA-ish format produced by `jit-core`/`aot-core`.
`IrProgram` is an untyped register-machine format consumed by all six `ir-to-*`
backends. The mapping is non-trivial:

1. **Variable-to-register assignment** — CIR uses named variables; IrProgram uses
   integer-indexed registers. A two-pass algorithm collects all names first, then
   emits instructions referencing the assigned indices.

2. **Type erasure** — CIR carries concrete types (`add_u8`, `add_i32`). IR is untyped:
   all integer arithmetic maps to `ADD`, `SUB`, etc. Float variants map to `F64_ADD`,
   etc.

3. **Missing opcodes** — IR has no `CMP_LE` or `CMP_GE` for integers. These are
   synthesised as two instructions: `CMP_GT + NOT` and `CMP_LT + NOT` respectively.
   Similarly, `neg_{int}` becomes `LOAD_IMM 0; SUB`.

4. **Unsupported ops** — `call_runtime`, `io_in`, `io_out` cannot be lowered without
   backend-specific knowledge. They raise `CIRLoweringError` in V1.

---

## Package: `cir-to-compiler-ir`

**Location:** `code/packages/python/cir-to-compiler-ir/`

```
cir-to-compiler-ir/
├── BUILD
├── CHANGELOG.md
├── README.md
├── pyproject.toml
└── src/
    └── cir_to_compiler_ir/
        ├── __init__.py     # exports: lower_cir_to_ir_program, validate_cir_for_lowering, CIRLoweringError
        ├── errors.py       # CIRLoweringError(Exception)
        ├── validator.py    # validate_cir_for_lowering(instrs) -> list[str]
        └── lowering.py     # _CIRLowerer + lower_cir_to_ir_program()
tests/
└── test_cir_to_compiler_ir.py
```

### Runtime dependencies

| Package | Why |
|---------|-----|
| `coding-adventures-compiler-ir` | `IrProgram`, `IrInstruction`, `IrOp`, `IDGenerator`, operand types |
| `coding-adventures-codegen-core` | `CIRInstr` |

---

## `CIRLoweringError`

A plain subclass of `Exception`. Raised by the lowerer when it encounters an
unsupported op (`call_runtime`, `io_in`, `io_out`) or an unknown op prefix.

```python
class CIRLoweringError(Exception):
    """Raised when lowering fails due to an unsupported CIR instruction."""
```

---

## `validate_cir_for_lowering(instrs: list[CIRInstr]) -> list[str]`

Validates a CIR instruction list before lowering. Returns a list of human-readable
error strings; empty list means the list is valid for lowering.

**Checks:**

| # | Check | Error message |
|---|-------|---------------|
| 1 | Empty list | `"empty instruction list"` |
| 2 | Any `call_runtime` op | `"unsupported op 'call_runtime' at index <i>: ..."` |
| 3 | Any `io_in` op | `"unsupported op 'io_in' at index <i>"` |
| 4 | Any `io_out` op | `"unsupported op 'io_out' at index <i>"` |
| 5 | Any `type == "any"` on arithmetic/comparison | `"unresolved type 'any' at index <i>: ..."` |

---

## `lower_cir_to_ir_program(instrs, entry_label="_start") -> IrProgram`

Lowers a `list[CIRInstr]` to an `IrProgram`. Raises `CIRLoweringError` on
unsupported ops. Calls `validate_cir_for_lowering` first; if any errors are
found, raises `CIRLoweringError` with the joined error messages.

### Two-pass algorithm

**Pass 1 — collect variables:**
Walk all `CIRInstr`. For every `dest` that is not `None`, and for every `str`
in `srcs`, register the name in `_reg: dict[str, int]` with a monotonically
increasing index. The order of first occurrence determines the register index.

**Pass 2 — emit IrInstructions:**
Walk `CIRInstr` in order, dispatch on `op` prefix, emit `IrInstruction` objects.

```python
prog = IrProgram(entry_label=entry_label)
# Always emit LABEL first (required by JVM/CIL backends)
prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel(entry_label)], id=-1))
gen = IDGenerator()
# ... emit translated instructions
```

### Register assignment helpers

```python
_reg: dict[str, int]  # variable name → register index
_next: int = 0        # next free index

def _var(name: str) -> IrRegister:
    """Look up or assign a register for a named variable."""
    if name not in _reg:
        _reg[name] = _next
        _next += 1
    return IrRegister(index=_reg[name])

def _fresh() -> IrRegister:
    """Allocate a scratch register (used for synthetic ops like neg/cmp_le)."""
    idx = _next
    _next += 1
    return IrRegister(index=idx)
```

### Literal helpers

CIR `srcs` entries are `str | int | float | bool`. The helper converts them:

- `str` → `IrRegister` (look up `_var`)
- `int` → `IrImmediate`
- `float` → `IrFloatImmediate`
- `bool` → `IrImmediate(1 if val else 0)`

### Op dispatch table

**Integer type suffix set:** `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `bool`
**Float type suffix set:** `f32`, `f64`

The lowerer strips the type suffix to identify the operation family, then
dispatches on the family name.

| CIR `op` pattern | IrOp emitted | Operands | Notes |
|---|---|---|---|
| `const_{int}` | `LOAD_IMM` | `[dest, IrImmediate(srcs[0])]` | bool True→1, False→0 |
| `const_f64` | `LOAD_F64_IMM` | `[dest, IrFloatImmediate(srcs[0])]` | |
| `const_bool` | `LOAD_IMM` | `[dest, IrImmediate(1 or 0)]` | |
| `add_{int}` | `ADD` | `[dest, src0, src1]` | |
| `sub_{int}` | `SUB` | `[dest, src0, src1]` | |
| `mul_{int}` | `MUL` | `[dest, src0, src1]` | |
| `div_{int}` | `DIV` | `[dest, src0, src1]` | |
| `and_{int}` | `AND` | `[dest, src0, src1]` | |
| `or_{int}` | `OR` | `[dest, src0, src1]` | |
| `xor_{int}` | `XOR` | `[dest, src0, src1]` | |
| `not_{int}` | `NOT` | `[dest, src0]` | single-input |
| `neg_{int}` | `LOAD_IMM` + `SUB` | scratch=0; `[dest, scratch, src0]` | 2 instrs |
| `add_f64` | `F64_ADD` | `[dest, src0, src1]` | |
| `sub_f64` | `F64_SUB` | `[dest, src0, src1]` | |
| `mul_f64` | `F64_MUL` | `[dest, src0, src1]` | |
| `div_f64` | `F64_DIV` | `[dest, src0, src1]` | |
| `neg_f64` | `LOAD_F64_IMM` + `F64_SUB` | scratch=0.0; `[dest, scratch, src0]` | 2 instrs |
| `cmp_eq_{int}` | `CMP_EQ` | `[dest, src0, src1]` | |
| `cmp_ne_{int}` | `CMP_NE` | `[dest, src0, src1]` | |
| `cmp_lt_{int}` | `CMP_LT` | `[dest, src0, src1]` | |
| `cmp_gt_{int}` | `CMP_GT` | `[dest, src0, src1]` | |
| `cmp_le_{int}` | `CMP_GT` + `NOT` | tmp=CMP_GT(src0,src1); `[dest, tmp]` | synthesised |
| `cmp_ge_{int}` | `CMP_LT` + `NOT` | tmp=CMP_LT(src0,src1); `[dest, tmp]` | synthesised |
| `cmp_eq_f64` | `F64_CMP_EQ` | `[dest, src0, src1]` | |
| `cmp_ne_f64` | `F64_CMP_NE` | `[dest, src0, src1]` | |
| `cmp_lt_f64` | `F64_CMP_LT` | `[dest, src0, src1]` | |
| `cmp_gt_f64` | `F64_CMP_GT` | `[dest, src0, src1]` | |
| `cmp_le_f64` | `F64_CMP_LE` | `[dest, src0, src1]` | direct (IrOp has LE for f64) |
| `cmp_ge_f64` | `F64_CMP_GE` | `[dest, src0, src1]` | direct (IrOp has GE for f64) |
| `label` | `LABEL` | `[IrLabel(srcs[0])]` | id=-1 |
| `jmp` | `JUMP` | `[IrLabel(srcs[0])]` | |
| `jmp_if_true` | `BRANCH_NZ` | `[src0_reg, IrLabel(srcs[1])]` | |
| `br_true_bool` | `BRANCH_NZ` | `[src0_reg, IrLabel(srcs[1])]` | |
| `jmp_if_false` | `BRANCH_Z` | `[src0_reg, IrLabel(srcs[1])]` | |
| `br_false_bool` | `BRANCH_Z` | `[src0_reg, IrLabel(srcs[1])]` | |
| `ret_void` | `HALT` | `[]` | return value ignored in V1 |
| `ret_{type}` | `HALT` | `[]` | return value ignored in V1 |
| `type_assert` | `COMMENT` | `[IrLabel("type_assert ...")]` | guard already fired in vm-core |
| `call` | `CALL` | `[IrLabel(srcs[0])]` | |
| `call_runtime` | raises `CIRLoweringError` | | unsupported in V1 |
| `io_in` | raises `CIRLoweringError` | | unsupported in V1 |
| `io_out` | raises `CIRLoweringError` | | unsupported in V1 |
| anything else | raises `CIRLoweringError` | | unknown op |

---

## Public API

```python
# __init__.py
from cir_to_compiler_ir.errors import CIRLoweringError
from cir_to_compiler_ir.validator import validate_cir_for_lowering
from cir_to_compiler_ir.lowering import lower_cir_to_ir_program

__all__ = ["CIRLoweringError", "validate_cir_for_lowering", "lower_cir_to_ir_program"]
```

---

## Invariants

1. The emitted `IrProgram` always begins with a `LABEL` pseudo-instruction for the
   entry label. This is required by JVM and CIL backends.
2. Every named variable in CIR receives a unique `IrRegister`. The same name always
   maps to the same register (single-assignment property is preserved).
3. Scratch registers (for synthesised ops) receive indices after all named variables
   have been assigned.
4. Instructions with `id=-1` are LABEL pseudo-instructions. All real instructions
   receive monotonically increasing non-negative IDs from `IDGenerator`.

---

## Limitations (V1)

- **Single function only.** The lowerer maps the entire `list[CIRInstr]` to a single
  `IrProgram`. Multi-function support (multiple entry points, call conventions) is
  deferred to LANG22.
- **Return values ignored.** `ret_{type}` emits `HALT` without capturing the return
  value. The return value is available in the source register but is not forwarded to
  the host. LANG22 will address this.
- **`call_runtime` unsupported.** Operations that require runtime dispatch (GC, dynamic
  dispatch, memory allocation) raise `CIRLoweringError`. LANG24 will add these.
- **`io_in`/`io_out` unsupported.** Backend-specific I/O lowering is deferred to LANG23.

---

## Testing

≥ 25 tests, ≥ 80% coverage. See `tests/test_cir_to_compiler_ir.py`.

Tests include:
- All arithmetic ops (integer and float)
- Synthesised ops (neg_int, cmp_le, cmp_ge)
- Control flow (label, jmp, branch_nz, branch_z)
- Type guards (type_assert → COMMENT)
- Calls
- Halt variants (ret_void, ret_i32)
- Error cases (call_runtime, io_in, io_out, unknown op)
- Validator (empty list, call_runtime, any type)
- Register reuse (same variable name → same register)
- Entry label is first instruction
- Round-trip through WASM validator (CIR → IrProgram → validate_for_wasm() == [])
- Round-trip through JVM validator (CIR → IrProgram → validate_for_jvm() == [])

---

## Out of scope (future LANGs)

- **LANG22:** Multi-function `IIRModule` → multi-entry `IrProgram`
- **LANG23:** `io_in`/`io_out` → SYSCALL mapping
- **LANG24:** `call_runtime` → generic dispatch / heap allocation
- **x86-64/AArch64:** Native backend — run directly on hardware
