## 0.52.0 — 2026-06-11 — McCarthy **lambda / LABEL / recursion** on real CoreCLR (CLR-real C5)

`compile_source_to_cil_text` now compiles multi-function McCarthy programs (via
`iir-to-cil-bytecode` 0.15.0 — multi-method emission, by-name `call`, `ldarg`
params, `castclass object[]` for object-typed field operands, `is_null`). New e2e
`tests/clr_real_lambda.rs` runs them on **real CoreCLR**: `((LAMBDA (X) X) 5)`→5,
`((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, a
COND-body lambda→100, and a recursive `LABEL` descending CARs→7 — plus symbol/
cons/scalar regression. Gated on `dotnet`+`ilasm`; skips when absent. This closes
the McCarthy F1–F7 feature set on the CLR's real runtime.

