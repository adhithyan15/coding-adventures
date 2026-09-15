## 0.191.0 — 2026-07-08 — ALGOL `for`-loop integer-array now proven on NativeAot + WASM (all 7 backends)

The ALGOL sum-of-squares `for`-loop-over-integer-array cell gains its **NativeAot** and
**WASM** columns, reaching all 7 backends. No backend change was needed: the `for`-loop
lowers to the same generic `alloc_array`/`array_get`/`array_set` + integer relation/
branch ops the straight-line E5 array cell already proved on both backends — the loop is
a pure control-flow composition over ops every backend lowers. Run-verified (WASM via the
in-repo `wasm-runtime`; NativeAot via a real executable in `run_native`).

