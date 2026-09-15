## 0.51.0 — 2026-06-11 — McCarthy **symbols** on real CoreCLR (CLR-real C4)

`compile_source_to_cil_text` now exercised on McCarthy symbol programs (via
`iir-to-cil-bytecode` 0.14.0 — no new ops; `intern_symbols_structural` turns each
`(QUOTE S)` into a tagged-int atom that reuses the existing const/box/equal? path).
New e2e `tests/clr_real_symbols.rs` runs them on **real CoreCLR**:
`(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`→0, `(ATOM (QUOTE A))`→1,
`(EQ (QUOTE FOO) (QUOTE FOO))`→1, `(EQ (QUOTE FOO) (QUOTE BAR))`→0, plus int-EQ /
cons regression. Gated on `dotnet`+`ilasm`; skips when absent.

