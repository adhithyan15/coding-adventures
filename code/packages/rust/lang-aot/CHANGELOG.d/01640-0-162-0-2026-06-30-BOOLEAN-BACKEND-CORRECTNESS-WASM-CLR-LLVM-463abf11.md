## 0.162.0 — 2026-06-30 — boolean backend correctness: WASM/CLR/LLVM fixes + matrix proof

Completed the `proven_columns_do_not_silently_skip` matrix test across all 7
backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

**lang_matrix.rs**:
- Added ALGOL compound boolean program (`s = 'ALPHA' and s != 'OMEGA'`) as a
  matrix Prog cell on all 7 backends — unblocked by WASM boolean and/or
  width-coherence fix and CLR Operand::Bool fix.
- Restored Llvm + Wasm to cells that were previously regressing (ALGOL `say`
  and string-echo programs).
- Improved assertion messages to include `p.src` on CLR failures for easier
  debugging.

**end_to_end_smoke.rs**:
- Updated `end_to_end_basic_print_emits_llvm_ir_with_print_extern` to assert
  `@__basic_print_real` instead of `@__print_i64` — the BASIC PRINT path
  changed in the BA2 overhaul (integer literals now route through the real
  formatter).

