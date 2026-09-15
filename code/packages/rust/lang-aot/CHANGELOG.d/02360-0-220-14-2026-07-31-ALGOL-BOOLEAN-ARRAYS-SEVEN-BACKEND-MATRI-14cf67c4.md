## 0.220.14 - 2026-07-31 (ALGOL boolean arrays — seven-backend matrix)

The matrix now executes a proper ALGOL procedure with a `boolean array` value
formal. It writes `true` and `false` through a non-unit-bound descriptor; the
caller reads those cells through `not` and exits with 42 on Native AOT, LLVM,
WASM, JVM, CLR, VM, and JIT.

