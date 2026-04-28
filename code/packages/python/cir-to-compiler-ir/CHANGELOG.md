# Changelog — coding-adventures-cir-to-compiler-ir

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-27

### Added

**LANG21: CIR-to-IrProgram lowering bridge.**

This release ships the first version of the `cir-to-compiler-ir` package,
closing the gap between the JIT/AOT specialisation pass and the six existing
`ir-to-*` backend code generators.

#### `lower_cir_to_ir_program(instrs, entry_label="_start") -> IrProgram`

The primary public function.  Converts a `list[CIRInstr]` produced by
`jit_core.specialise()` or `aot_core.aot_specialise()` into a fully-formed
`IrProgram` ready for any `ir-to-*` backend.

Algorithm:
- **Pass 1** — collects every named variable from `dest` and `srcs` fields
  and assigns a monotonically increasing `IrRegister` index in order of
  first occurrence.
- **Pass 2** — walks the instruction list and emits `IrInstruction` objects,
  using `IDGenerator` for unique IDs on real instructions.

Always emits a `LABEL` pseudo-instruction at index 0 (required by the JVM
and CIL backends).

#### `validate_cir_for_lowering(instrs) -> list[str]`

Pre-lowering validator that returns all validation errors at once:
- Empty instruction list
- `call_runtime` ops (unsupported in V1)
- `io_in` / `io_out` ops (backend-specific, unsupported in V1)
- `type == "any"` on arithmetic/comparison ops (specialisation did not resolve)

#### `CIRLoweringError`

Plain `Exception` subclass raised when lowering fails due to an unsupported
or unknown CIR op.

#### Op mapping (complete)

| CIR op family | IrOp(s) emitted | Notes |
|---|---|---|
| `const_{int}` | `LOAD_IMM` | bool True→1, False→0 |
| `const_f64` | `LOAD_F64_IMM` | |
| `add/sub/mul/div_{int}` | `ADD/SUB/MUL/DIV` | |
| `and/or/xor_{int}` | `AND/OR/XOR` | |
| `not_{int}` | `NOT` | single-source |
| `neg_{int}` | `LOAD_IMM(0)` + `SUB` | 2 instructions |
| `add/sub/mul/div_f64` | `F64_ADD/SUB/MUL/DIV` | |
| `neg_f64` | `LOAD_F64_IMM(0.0)` + `F64_SUB` | 2 instructions |
| `cmp_eq/ne/lt/gt_{int}` | `CMP_EQ/NE/LT/GT` | |
| `cmp_le_{int}` | `CMP_GT` + `NOT` | synthesised (no CMP_LE in IrOp) |
| `cmp_ge_{int}` | `CMP_LT` + `NOT` | synthesised (no CMP_GE in IrOp) |
| `cmp_eq/ne/lt/gt/le/ge_f64` | `F64_CMP_*` | all 6 direct (IrOp has them) |
| `label` | `LABEL` | id=-1 |
| `jmp` | `JUMP` | |
| `jmp_if_true` / `br_true_bool` | `BRANCH_NZ` | |
| `jmp_if_false` / `br_false_bool` | `BRANCH_Z` | |
| `ret_void` / `ret_{type}` | `HALT` | return value ignored in V1 |
| `type_assert` | `COMMENT` | guard already fired in vm-core |
| `call` | `CALL` | |
| `call_runtime` | `CIRLoweringError` | unsupported in V1 |
| `io_in` / `io_out` | `CIRLoweringError` | unsupported in V1 |

#### Tests

48 tests covering:
- All arithmetic and logical ops (integer and float)
- Synthesised ops (neg_int, cmp_le_int, cmp_ge_int)
- All six float comparisons (direct IrOp mapping)
- All control flow ops (label, jmp, branch_nz, branch_z, call)
- Type assertions → COMMENT
- Return and halt variants
- All error cases (call_runtime, io_in, io_out, unknown op)
- Validator (empty list, multiple errors, control-flow any-type exemption)
- Register reuse invariant (same name → same register)
- Entry label position and custom entry label
- Instruction ID invariants (labels=-1, real≥0)
- Round-trip: CIR → IrProgram → `validate_for_wasm()` == []
- Round-trip: CIR → IrProgram → `validate_for_jvm()` == []

Coverage: > 80%.
