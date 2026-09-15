## 0.52.0 — 2026-08-01 — runtime integer exponents

ALGOL exponentiation now lowers every numeric pair that is not a nonnegative
integer-literal fast path through `f64_pow`. Integer bases therefore support runtime and
negative integer exponents by widening to scalar `f64` results, preserving reciprocal
powers without compile-time multiplication expansion.

