## 0.220.23 - 2026-08-01 (forwarded captured 4-D ALGOL real arrays — seven-backend matrix)

The matrix now executes a nested ALGOL procedure that forwards a captured,
non-unit-bound four-dimensional `real array` value formal to a sibling array formal.
A caller-side floating-point sum proves the `array<f64>` handle, all four lower bounds,
and all three row-major strides survived two procedure boundaries on Native AOT, LLVM,
WASM, JVM, CLR, VM, and JIT.

