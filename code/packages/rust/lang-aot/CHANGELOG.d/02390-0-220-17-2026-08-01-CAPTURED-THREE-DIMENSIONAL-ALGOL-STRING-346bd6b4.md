## 0.220.17 - 2026-08-01 (captured three-dimensional ALGOL string arrays — seven-backend matrix)

The matrix now executes a nested ALGOL procedure that writes a three-dimensional,
non-unit-bound `string array` value formal. Its caller verifies lexical ordering,
equality, and inequality of distinct cells, proving the dynamic `array<str>` descriptor
retains all three lower bounds and both row-major strides on Native AOT, LLVM, WASM,
JVM, CLR, VM, and JIT.

