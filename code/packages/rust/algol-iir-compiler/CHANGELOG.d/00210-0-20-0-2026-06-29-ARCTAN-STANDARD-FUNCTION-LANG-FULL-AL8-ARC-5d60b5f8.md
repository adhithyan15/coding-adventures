## 0.20.0 — 2026-06-29 — `arctan` standard function (LANG-FULL AL8-arctan)

The ALGOL 60 §3.2.4 `arctan` standard function is now recognised and lowered to the
`f64_atan` IIR op via the existing `emit_f64_unary` helper:

- `arctan(E)` → `f64_atan` (libm `atan` on native/LLVM; `env.__atan` on WASM;
  `Math.atan` on JVM; `System.Math.Atan` on CLR; `f64::atan` on VM/JIT)

The inverse tangent maps −∞..+∞ → −π/2..π/2.  `arctan(0.0)` = 0.0 exactly in
IEEE-754 double, used as the matrix proof value.  Completes the transcendental standard
functions listed in ALGOL 60 §3.2.4.

