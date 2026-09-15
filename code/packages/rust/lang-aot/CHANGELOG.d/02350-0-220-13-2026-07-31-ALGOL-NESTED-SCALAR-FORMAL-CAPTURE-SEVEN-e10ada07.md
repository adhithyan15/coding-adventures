## 0.220.13 - 2026-07-31 (ALGOL nested scalar-formal capture — seven-backend matrix)

The matrix now executes an ALGOL procedure whose nested sibling increments an
enclosing scalar `value` parameter. The outer frame publishes the typed value
to shared global storage before the nested call; the program returns 42 on
Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

