## 0.220.16 - 2026-08-01 (captured multidimensional ALGOL string arrays — seven-backend matrix)

The matrix now executes a nested ALGOL procedure that writes a two-dimensional,
non-unit-bound `string array` value formal. Its caller verifies lexical ordering,
equality, and inequality of distinct cells, proving the dynamic `array<str>` descriptor
survives capture on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

