## 0.220.15 - 2026-08-01 (multidimensional ALGOL boolean arrays — seven-backend matrix)

The matrix now runs a two-dimensional, non-unit-bound boolean array through a
value formal. Its checkerboard writes prove both lower bounds and the outer
row-major stride survive the call on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

