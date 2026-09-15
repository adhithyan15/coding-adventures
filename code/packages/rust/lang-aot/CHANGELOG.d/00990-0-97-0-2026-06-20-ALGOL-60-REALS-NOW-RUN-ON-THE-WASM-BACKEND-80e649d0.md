## 0.97.0 — 2026-06-20 — ALGOL 60 reals now run on the WASM backend too (LANG-FULL E3-codegen-slots, WASM)

The two ALGOL 60 **real** matrix programs now also run on the **WASM** column
(LLVM + WASM + VM + JIT). WASM needed **no backend change**: `iir-to-wasm`'s
typed-local model already carries an `f64` variable in an `F64` local and
selects `f64.mul`/`f64.eq`/`f64.lt` from the `f64` type_hint — the
uniform-`i64`-slot problem that LLVM had simply doesn't exist there. The
`proven_columns_do_not_silently_skip` guard confirms `wasm-runtime` actually
executes the programs. (`iir-to-wasm` 0.15.1 adds op-selection regression
tests.) Remaining E3: jvm (slot fix), native + CLR (FP emission).

