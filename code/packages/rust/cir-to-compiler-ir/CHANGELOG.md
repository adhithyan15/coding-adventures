# Changelog — cir-to-compiler-ir

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-28

### Added

- **`CIRLoweringError`** — concrete, cloneable, `std::error::Error`-implementing
  error type returned by all fallible functions.
- **`validate_cir_for_lowering(instrs: &[CIRInstr]) -> Vec<String>`** — pre-lowering
  safety checker: rejects empty lists, `call_runtime`, `io_in`/`io_out`, and
  instructions with unresolved `type == "any"`.
- **`lower_cir_to_ir_program(instrs, entry_label) -> Result<IrProgram, CIRLoweringError>`**
  — two-pass CIR → IrProgram lowering:
  - **Pass 1**: collects all variable names and assigns stable virtual register indices.
  - **Pass 2**: emits IR instructions in order, dispatching on `op` prefix.
- Supported CIR ops: `const_*`, `add_*`, `sub_*`, `and_*`, `neg_*`, `cmp_eq_*`,
  `cmp_ne_*`, `cmp_lt_*`, `cmp_gt_*`, `cmp_le_*` (synthesised), `cmp_ge_*`
  (synthesised), `label`, `jmp`, `jmp_if_true`, `jmp_if_false`, `br_true_bool`,
  `br_false_bool`, `ret_void`, `ret_*`, `type_assert`, `call`, `tetrad.move`,
  `load_mem`, `store_mem`, `call_builtin`.
- `cmp_le` synthesis: `1 − CmpGt(a, b)` (avoids the missing `NOT` op).
- `cmp_ge` synthesis: `1 − CmpLt(a, b)`.
- `neg` synthesis: `LoadImm(0)` + `Sub(0, src)`.
- Unsupported ops that return `CIRLoweringError`: `mul_*`, `div_*`, `or_*`,
  `xor_*`, `not_*`, all float ops (`f32`/`f64`), `call_runtime`, `io_in`,
  `io_out`, and unknown ops.
- 56 unit tests across all three modules (errors, validator, lowering).
- Literate-programming-style inline documentation throughout.
