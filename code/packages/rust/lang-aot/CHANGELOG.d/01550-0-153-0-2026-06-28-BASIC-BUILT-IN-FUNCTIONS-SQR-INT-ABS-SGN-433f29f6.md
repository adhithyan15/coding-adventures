## 0.153.0 — 2026-06-28 — BASIC built-in functions `SQR`/`INT`/`ABS`/`SGN` (LANG-FULL BA-builtins)

Four new matrix `Prog` cells proving Dartmouth BASIC's built-in math functions
on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- `PRINT SQR(49)` → `7` (`f64_sqrt` IIR op, hardware sqrt everywhere)
- `PRINT INT(3.7)` → `3` (`real_to_int_floor` + `int_to_real`, E8 ops)
- `PRINT ABS(-42)` → `42` (inline conditional, store-per-branch)
- `PRINT SGN(-5)` → `-1` (inline 3-way conditional, result is f64)

818 total proven cells pass.

