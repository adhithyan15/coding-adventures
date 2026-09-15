## 0.50.0 — 2026-06-11 — McCarthy **predicates + COND** on real CoreCLR (CLR-real C3)

`compile_source_to_cil_text` now compiles McCarthy predicate and `COND` programs
(via `iir-to-cil-bytecode` 0.13.0). New e2e `tests/clr_real_predicates.rs` runs them
on **real CoreCLR** (real `ilasm` → PE → real `dotnet`, via the shared
`tests/clr_support` harness): `(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1,
`(EQ 7 8)`→0, `(COND ((ATOM 7) 11) ((ATOM 8) 22))`→11, and
`(COND ((ATOM (CONS 1 2)) 11) ((EQ 5 5) 22))`→22 (a false first clause falling
through to a matching `EQ`). Gated on `dotnet`+`ilasm`; skips when absent.

