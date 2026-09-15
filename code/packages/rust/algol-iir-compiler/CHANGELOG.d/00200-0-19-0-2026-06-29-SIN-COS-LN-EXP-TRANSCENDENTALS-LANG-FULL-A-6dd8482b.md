## 0.19.0 — 2026-06-29 — `sin`/`cos`/`ln`/`exp` transcendentals (LANG-FULL AL8-trig)

The four ALGOL 60 §3.2.4 transcendental standard functions are now recognised and
lowered to the new `f64_sin`, `f64_cos`, `f64_ln`, `f64_exp` IIR ops:

- `sin(E)` → `f64_sin` (dispatch to libm `sin` on native, `env.__sin` on WASM,
  `@llvm.sin.f64` on LLVM, `Math.sin` on JVM, `System.Math.Sin` on CLR, `f64::sin` on VM/JIT)
- `cos(E)` → `f64_cos` (same pattern with `cos`)
- `ln(E)`  → `f64_ln`  (ALGOL uses `ln` for natural log; backends use `log`/`log.f64`)
- `exp(E)` → `f64_exp` (same pattern with `exp`)

Each op is implemented by the new `emit_f64_unary` helper (mirrors `emit_sqrt`).
All require exactly one `real`-typed argument; wrong arity or type → `CompileError::Type`.
Proof programs exit 42 on all 7 backends.

