## 0.35.0 — 2026-06-10 — McCarthy → **LLVM** scalar run-foundation via `clang` (LANG77 / W12a)

Establishes the LLVM **verify-by-running** substrate — the first **tagged-word**
target (the LLVM/AOT/JIT family that links the shared `lispy_runtime.c`). New
`compile_source_to_llvm` / `compile_source_to_llvm_with_target`: concretize scalar
`any`→`i64`, lower to LLVM IR text. The new `tests/llvm_scalar.rs` emits **host**-
triple IR (`clang -dumpmachine`), builds it with `clang -x ir`, and **runs** the
native executable — its process exit code carries the McCarthy result: `42`→42,
`7`→7, `0`→0, `100`→100 (+ Twig `42`→42). Uses the `clang` already on the box (no
extra toolchain; self-skips if absent) — the LLVM analogue of `wasm-runtime` /
`clr-simulator` / real `erl`. The cons/predicate/symbol/lambda lowering
(`call __twig_lispy_*`) is W12b+.

