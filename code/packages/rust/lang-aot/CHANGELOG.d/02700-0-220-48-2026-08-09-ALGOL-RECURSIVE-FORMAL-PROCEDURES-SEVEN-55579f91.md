## 0.220.48 - 2026-08-09 (ALGOL recursive formal procedures — seven-backend matrix)

The matrix now recursively forwards `twice` through an ALGOL `procedure`
formal on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The active specialised
`descend` sibling reuses itself until its base case invokes `twice` directly,
returning 42 without a runtime procedure descriptor.

