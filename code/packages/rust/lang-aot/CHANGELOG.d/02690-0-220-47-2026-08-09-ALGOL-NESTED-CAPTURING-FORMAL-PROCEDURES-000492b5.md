## 0.220.47 - 2026-08-09 (ALGOL nested capturing formal procedures — seven-backend matrix)

The matrix now passes a nested `add` procedure, which captures its enclosing
`seed` value formal, through an ALGOL `procedure` formal on Native AOT, LLVM,
WASM, JVM, CLR, VM, and JIT. It returns 42 through the direct specialised call
path without a closure descriptor.

