## 0.220.20 - 2026-08-01 (captured four-dimensional ALGOL integer arrays — seven-backend matrix)

The matrix now executes a nested ALGOL procedure that writes a four-dimensional,
non-unit-bound `integer array` value formal. A caller-side corner sum proves the dynamic
`array<i64>` descriptor retains all four lower bounds and all three row-major strides on
Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

