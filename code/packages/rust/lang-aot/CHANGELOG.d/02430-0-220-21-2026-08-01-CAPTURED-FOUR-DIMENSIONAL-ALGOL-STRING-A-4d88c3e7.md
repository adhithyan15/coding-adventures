## 0.220.21 - 2026-08-01 (captured four-dimensional ALGOL string arrays — seven-backend matrix)

The matrix now executes a nested ALGOL procedure that writes a four-dimensional,
non-unit-bound `string array` value formal. Its lexical ordering, equality, and
inequality checks prove the dynamic `array<str>` descriptor retains all four lower
bounds and all three row-major strides on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

