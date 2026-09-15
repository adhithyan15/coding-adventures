## 0.36.0 — 2026-07-31 — AL1 integer-to-real promotion

`integer` values now widen through the shared `int_to_real` IIR conversion
when a `real` is required. This covers mixed integer/real arithmetic and
comparisons, real division, real scalar and array-element assignments, real
value parameters, real standard-function arguments, and a real exponent for
`^`. `div` and `mod` remain integer-only; nonnumeric mismatches remain type
errors.

The compiler regressions cover each conversion site, and the LANG matrix runs
an ALGOL program through Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

