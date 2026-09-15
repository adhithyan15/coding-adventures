## 0.101.0 — 2026-06-21 — ALGOL arrays run on the JVM (LANG-FULL E5 PR-3)

`concretize_scalar_any_for_jvm` now narrows an `array<i64>` handle to
`array<i32>` in lockstep with the existing scalar `i64`→`i32` narrowing. Without
this the JVM backend would build a `long[]` (from `alloc_array`'s `array<i64>`)
but emit `iaload`/`iastore` (from the now-`i32` element hints) — a `long[]` with
`iaload` is a real-`java` `VerifyError`. Aligning the element type makes the
whole array `int[]` and lets the ALGOL sum-of-squares array `Prog` run on the
**JVM** column (`lang_matrix.rs`, exit 55) alongside VM + JIT. Pairs with
`iir-to-jvm-class-file` 0.16.0 (native `int[]`/`long[]`/`double[]` lowering).

