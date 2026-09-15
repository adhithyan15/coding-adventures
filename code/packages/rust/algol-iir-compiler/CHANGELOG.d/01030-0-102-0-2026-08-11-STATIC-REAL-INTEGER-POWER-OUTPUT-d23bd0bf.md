## 0.102.0 — 2026-08-11 — static real integer-power output

Finite literal-only real bases raised to small nonnegative integer-literal
exponent chains now print through the bounded compile-time string path. The
existing exponent cap and repeated-multiplication semantics avoid host `pow`
variation; real, negative, oversized, runtime, and non-finite powers fail closed.

