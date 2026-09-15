## 0.220.18 - 2026-08-01 (captured three-dimensional ALGOL boolean arrays — seven-backend matrix)

The matrix now executes a nested ALGOL procedure that writes a three-dimensional,
non-unit-bound `boolean array` value formal. Its checkerboard reads prove the dynamic
`array<bool>` descriptor retains all three lower bounds and both row-major strides on
Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

