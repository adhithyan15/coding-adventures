## 0.12.0 — 2026-06-03 — Phase 7 (FINAL lane) of historical-arch backend migration

### What changed

`--emit=riscv32` now routes through `aot_core::infer` +
`aot_core::specialise` + `riscv_backend::compile` (the new
`Backend` trait implementation) instead of `iir_to_riscv`
(deprecated as of v0.4.0).

Same pattern as the previous five migration phases for GE-225
(Phase 3), Intel 4004 (Phase 4), ARMv7 (Phase 5), and Intel 8008
(Phase 6).

### Migration complete

With Phase 7 landed, every historical-arch lane now consumes typed
CIR via the `Backend` trait.  The historical migration is **done**.

See `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md` for the full
end-state summary.

### Dependencies

* Removed: `iir-to-riscv` (deprecated; lang-aot no longer pulls
  it in).
* Added: `riscv-encoder`, `riscv-backend`.

### Test surface

* The existing `end_to_end_basic_print_emits_riscv32_bin_via_lang_aot`
  test now exercises the new CIR-via-Backend path.  BASIC
  `PRINT 42` lowers `call_builtin print_i64`, which the v0.1.0
  `riscv-backend` doesn't yet cover — the test treats that as an
  expected gap and skips with `eprintln!`, identical to its
  behaviour during Phases 5 and 6.

