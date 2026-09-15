## 0.152.0 — 2026-06-28 — ALGOL `sqrt` on all seven backends (LANG-FULL AL8-sqrt)

The matrix now proves `sqrt(49.0) = 7` — the ALGOL 60 §3.2.4 `sqrt` standard
function — on **all 7 backends** (NativeAot / LLVM / WASM / JVM / CLR / VM / JIT).
The proof program is `begin real r; integer result; r := sqrt(49.0); result :=
entier(r) end` → exit 7.  `sqrt` lowers through the new `f64_sqrt` IIR op to
hardware sqrt on every backend: WASM `f64.sqrt` (0x9F), LLVM `@llvm.sqrt.f64`,
JVM `Math.sqrt`, CLR `System.Math::Sqrt`, aarch64 `FSQRT`, x86_64 `SQRTSD`,
VM/JIT `f64::sqrt()`.

