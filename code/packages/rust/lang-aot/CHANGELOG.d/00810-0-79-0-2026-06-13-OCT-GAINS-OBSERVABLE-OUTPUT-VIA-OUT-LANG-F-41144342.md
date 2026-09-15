## 0.79.0 — 2026-06-13 — Oct gains observable output via `out` (LANG-FULL O-OUT)

`tests/lang_matrix.rs` gains two executed Oct programs that print to **stdout** —
`out(1, 200)` → `200` and `out(1, 100 + 100)` → `200` (arithmetic proven observably)
— across native / LLVM / WASM / JVM / CLR / VM / JIT. This is Oct's first checkable
output: until now Oct's void `main` always exited 0, so no Oct result could be
verified by running. Backed by `oct-iir-compiler` 0.5.0, which lowers the 8008 `out`
intrinsic to `call_builtin "print_i64"`. Unblocks verification of the Oct value-level
items. No `lang-aot/src` change.

