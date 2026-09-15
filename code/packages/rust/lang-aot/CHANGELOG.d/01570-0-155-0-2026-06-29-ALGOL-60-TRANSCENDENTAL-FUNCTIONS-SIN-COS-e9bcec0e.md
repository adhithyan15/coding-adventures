## 0.155.0 — 2026-06-29 — ALGOL 60 transcendental functions `sin`/`cos`/`ln`/`exp` (LANG-FULL AL8-trig)

Four new matrix `Prog` cells proving ALGOL 60's §3.2.4 transcendental standard
functions on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- `cos(0.0)` → `entier(1.0) + 41` = 42
- `exp(0.0)` → `entier(1.0) + 41` = 42
- `sin(0.0)` → `entier(0.0 + 42.0)` = 42
- `ln(1.0)`  → `entier(0.0 + 42.0)` = 42

Each uses an exact IEEE-754 input/output (no rounding error) to verify correctness
portably.  Backend mappings: WASM `env.__sin/cos/ln/exp` host imports (resolved by
`PrintHost` to Rust `f64::*`); LLVM `@llvm.sin/cos/log/exp.f64` intrinsics;
JVM `Math.sin/cos/log/exp`; CLR `System.Math.Sin/Cos/Log/Exp`; native aarch64/x86_64
`BL` / `call rel32` to libm `sin/cos/log/exp`; VM/JIT `f64_*` dispatch handlers.

Four new WASM host functions added to `PrintHost`: `SinFunc`, `CosFunc`, `LnFunc`,
`ExpFunc` — each stateless, `f64 → f64`, resolved as `env.__sin/cos/ln/exp`.

842 total proven cells pass.

