# cir-to-compiler-ir

> LANG21 bridge: converts `list[CIRInstr]` → `IrProgram` so that any Tetrad
> JIT/AOT-compiled function can be routed through any of the six existing
> `ir-to-*` backends (WASM, JVM, CIL, GE-225, Intel 4004, Intel 8008).

---

## Where this fits in the pipeline

```
Tetrad source
  → tetrad-lexer / parser / type-checker / compiler → CodeObject
  → tetrad-runtime.compile_to_iir()                 → IIRModule
  → jit-core.specialise(fn, min_observations=0)     → list[CIRInstr]
  → CIROptimizer.run()                              → list[CIRInstr]
  → lower_cir_to_ir_program()   ← THIS PACKAGE
  → IrProgram
  → WASMCodeGenerator / JVMCodeGenerator / ...      ← LANG20 adapters
  → WasmModule / JVMClassArtifact / ...
  → wasm-runtime / GraalVM / GE-225 simulator / ...
```

`CIRInstr` is a typed, SSA-shaped instruction produced by the JIT/AOT
specialisation pass.  `IrProgram` is the untyped register-machine IR
consumed by every `ir-to-*` backend.

This package provides the lowering pass that bridges the two.

---

## Installation

```bash
pip install coding-adventures-cir-to-compiler-ir
```

Or, from source (editable):

```bash
pip install -e path/to/cir-to-compiler-ir
```

---

## Quick start

```python
from codegen_core import CIRInstr
from cir_to_compiler_ir import lower_cir_to_ir_program
from ir_to_wasm_compiler import WASMCodeGenerator

# A tiny Tetrad-like CIR program: z = x + y
instrs = [
    CIRInstr("const_i32", "x", [40], "i32"),
    CIRInstr("const_i32", "y", [2],  "i32"),
    CIRInstr("add_i32",   "z", ["x", "y"], "i32"),
    CIRInstr("ret_void",  None, [], "void"),
]

# Lower to IrProgram
prog = lower_cir_to_ir_program(instrs)

# Compile to WASM via the LANG20 CodeGenerator adapter
gen = WASMCodeGenerator()
errors = gen.validate(prog)
assert errors == []
wasm_module = gen.generate(prog)
```

---

## API reference

### `lower_cir_to_ir_program(instrs, entry_label="_start") -> IrProgram`

The main function.  Validates `instrs` then lowers them to `IrProgram`.

- **`instrs`** — `list[CIRInstr]` from `jit_core.specialise()` or
  `aot_core.aot_specialise()`
- **`entry_label`** — name of the entry-point label (default `"_start"`)
- **Raises** `CIRLoweringError` if validation fails or an unsupported op
  is encountered

### `validate_cir_for_lowering(instrs) -> list[str]`

Returns all validation errors without raising.  Empty list → valid.

Checks:
- Empty instruction list
- `call_runtime` ops (unsupported in V1)
- `io_in` / `io_out` ops (unsupported in V1)
- `type == "any"` on arithmetic/comparison ops (specialisation did not resolve)

### `CIRLoweringError`

Subclass of `Exception` raised when lowering fails.

---

## Op mapping

| CIR family | IrOp(s) | Notes |
|---|---|---|
| `const_{int_type}` | `LOAD_IMM` | bool True→1, False→0 |
| `const_f64` | `LOAD_F64_IMM` | |
| `add/sub/mul/div_{int}` | `ADD/SUB/MUL/DIV` | type erased |
| `and/or/xor_{int}` | `AND/OR/XOR` | |
| `not_{int}` | `NOT` | |
| `neg_{int}` | `LOAD_IMM(0)` + `SUB` | synthesised — 2 instructions |
| `add/sub/mul/div_f64` | `F64_ADD/SUB/MUL/DIV` | |
| `neg_f64` | `LOAD_F64_IMM(0.0)` + `F64_SUB` | synthesised — 2 instructions |
| `cmp_eq/ne/lt/gt_{int}` | `CMP_EQ/NE/LT/GT` | |
| `cmp_le_{int}` | `CMP_GT` + `NOT` | synthesised (IrOp has no CMP_LE) |
| `cmp_ge_{int}` | `CMP_LT` + `NOT` | synthesised (IrOp has no CMP_GE) |
| `cmp_eq/ne/lt/gt/le/ge_f64` | `F64_CMP_*` | all 6 map directly |
| `label` | `LABEL` | id = -1 |
| `jmp` | `JUMP` | |
| `jmp_if_true` / `br_true_bool` | `BRANCH_NZ` | |
| `jmp_if_false` / `br_false_bool` | `BRANCH_Z` | |
| `ret_void` / `ret_{type}` | `HALT` | return value ignored in V1 |
| `type_assert` | `COMMENT` | guard already checked in vm-core |
| `call` | `CALL` | |
| `call_runtime` | **raises** | unsupported in V1 |
| `io_in` / `io_out` | **raises** | unsupported in V1 |

---

## Limitations (V1)

- **Single function only.** The entire `list[CIRInstr]` maps to one `IrProgram`.
  Multi-function support is planned for LANG22.
- **Return values ignored.** `ret_{type}` emits `HALT`; the value in the source
  register is not forwarded to the host.  LANG22 will address this.
- **`call_runtime` unsupported.** GC allocation, dynamic dispatch, and FFI raise
  `CIRLoweringError`.  LANG24 will add these.
- **`io_in`/`io_out` unsupported.** Platform I/O lowering is deferred to LANG23.

---

## Related packages

| Package | Role |
|---------|------|
| `coding-adventures-codegen-core` | Defines `CIRInstr` (the input format) |
| `coding-adventures-compiler-ir` | Defines `IrProgram` (the output format) |
| `coding-adventures-ir-to-wasm-compiler` | WASM backend (LANG20 adapter) |
| `coding-adventures-ir-to-jvm-class-file` | JVM backend (LANG20 adapter) |
| `coding-adventures-jit-core` | Produces `list[CIRInstr]` via specialisation |

---

## Spec

See [`code/specs/LANG21-cir-to-compiler-ir.md`](../../../../specs/LANG21-cir-to-compiler-ir.md)
for the full design rationale, op mapping table, and out-of-scope items.
