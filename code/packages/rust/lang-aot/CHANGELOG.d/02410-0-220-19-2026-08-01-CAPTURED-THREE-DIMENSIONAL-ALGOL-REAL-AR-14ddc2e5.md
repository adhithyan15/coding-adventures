## 0.220.19 - 2026-08-01 (captured three-dimensional ALGOL real arrays — seven-backend matrix)

The matrix now executes a nested ALGOL procedure that writes a three-dimensional,
non-unit-bound `real array` value formal. A caller-side floating-point sum proves the
dynamic `array<f64>` descriptor retains all three lower bounds and both row-major strides
on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

