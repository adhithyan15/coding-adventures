## 0.220.25 - 2026-08-01 (forwarded captured 4-D ALGOL boolean arrays — seven-backend matrix)

The matrix now executes a nested ALGOL procedure that forwards a captured,
non-unit-bound four-dimensional `boolean array` value formal to a sibling array formal.
Caller-side checkerboard reads prove the `array<bool>` handle, all four lower bounds,
and all three row-major strides survived two procedure boundaries on Native AOT, LLVM,
WASM, JVM, CLR, VM, and JIT.

