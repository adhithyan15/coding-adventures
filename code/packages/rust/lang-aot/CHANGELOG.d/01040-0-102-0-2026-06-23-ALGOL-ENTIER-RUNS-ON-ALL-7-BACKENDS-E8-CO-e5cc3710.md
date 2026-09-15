## 0.102.0 — 2026-06-23 — ALGOL `entier` runs on ALL 7 backends — E8 COMPLETE (LANG-FULL E8 PR-7)

`tests/lang_matrix.rs` gains an executed ALGOL `entier` program —
`begin real r; integer result; r := 0.0 - 2.7; result := 45 + entier(r) end`
⇒ exit **42** — across native-AOT + LLVM + WASM + JVM + CLR + VM + JIT. `entier(E)`
(ALGOL 60 §3.2.5) is the largest integer not greater than the *real* `E` — floor,
rounding toward −∞ — and lowers to a single E8 `real_to_int_floor` IIR op
(`algol-iir-compiler` 0.10.0), so every backend emits its native floor-then-convert
(LLVM `@llvm.floor.f64`+`fptosi`, WASM `f64.floor`+`i32.trunc_f64_s`, JVM
`Math.floor`+`d2i`, CLR `Math::Floor`+`conv.ovf.i4`, native aarch64 `frintm`+`fcvtzs`,
native x86_64 `roundsd …,1`+`cvttsd2si`). The program builds a **negative** real so
the result distinguishes floor from trunc: `45 + entier(−2.7)` = `45 + (−3)` = 42
(trunc would give 43). `run_native` compiles for the host arch, so the `NativeAot`
cell is executed on aarch64 locally (Apple Silicon). This is the proof that **closes
the E8 numeric-conversions arc** — `int_to_real`/`real_to_int_trunc`/`real_to_int_floor`
now run end-to-end on all seven backends.

