## 0.158.0 — 2026-06-29 — BASIC `^` general exponentiation on 7 backends (LANG-FULL BA-pow)

Added `PowFunc` WASM host function (`env.__pow(f64, f64) -> f64`, uses
`f64::powf`) and registered it in `PrintHost::resolve_function`.  Added proof
cell: `PRINT 4 ^ 0.5` → `Stdout("2")` on all 7 backends (NativeAot, Llvm,
Wasm, Jvm, Clr, Vm, Jit).  `pow(4.0, 0.5) = 2.0` exactly; the `2` output
(no decimal point) comes from `__basic_print_real`'s existing integer-valued
real formatting.

849 total proven cells pass.
