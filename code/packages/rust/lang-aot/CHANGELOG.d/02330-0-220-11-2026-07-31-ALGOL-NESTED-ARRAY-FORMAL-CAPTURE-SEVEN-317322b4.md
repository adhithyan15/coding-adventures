## 0.220.11 - 2026-07-31 (ALGOL nested array-formal capture — seven-backend matrix)

The matrix now executes a nested ALGOL procedure which writes a two-dimensional
array `value` formal from its enclosing procedure. The generated descriptor
globals preserve the shared handle, nonzero lower bounds, and outer stride; the
program returns 42 on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

