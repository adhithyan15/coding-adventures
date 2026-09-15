## 0.163.0 — 2026-06-30 — BA-INPUT: JVM `INPUT X` end-to-end (concretize + test matrix)

**`concretize_scalar_any_for_jvm`** (`src/lib.rs`): The function that narrows
`i64` type-hints to `i32` for programs that don't need the JVM wide `long` model
previously only exempted programs that call `print_i64` from narrowing.  BASIC
`INPUT X` programs call `input_i64` but not `print_i64`, so they were incorrectly
narrowed to `i32` — causing a JVM VerifyError when `readLong()J` tried to store a
`long` into an `int` slot.  Added `WIDE_I64_BUILTINS = ["print_i64", "input_i64"]`
and updated both the module-level `module_prints` check and the per-function
`uses_wide_builtin` guard to use this list.

**`putchar` lowering** (`iir-to-jvm-class-file`): In wide i64 mode, the value
being written via `putchar(I)V` lives in a Long slot.  The lowering previously
always emitted `iload` (int load), causing a VerifyError when the slot contained a
`long`.  Now emits `lload; l2i` when `val_type == JvmType::Long`.

**`emit_lconst_cp`** (`iir-to-jvm-class-file`): Added proper Long CP-entry helper
for integer literals outside the i16 range stored into `JvmType::Long` slots (e.g.
`100000` in `__basic_print_real`).  Fixed the `"const"` and `"return"` IIR cases to
call it.

**Test matrix** (`tests/lang_matrix.rs`): Added two new Dartmouth BASIC INPUT
`Prog` entries:
- `10 INPUT X\n20 PRINT X\n30 END` with stdin `42` → stdout `"42"` (single INPUT)
- `10 INPUT A\n20 INPUT B\n30 PRINT A+B\n40 END` with stdin `10\n32` → `"42"` (two INPUTs)

`matrix_every_proven_cell_agrees` and `proven_columns_do_not_silently_skip` both
pass across all 7 backends.

