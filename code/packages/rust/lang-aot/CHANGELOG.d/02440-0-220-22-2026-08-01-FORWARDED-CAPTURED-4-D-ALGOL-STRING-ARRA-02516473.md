## 0.220.22 - 2026-08-01 (forwarded captured 4-D ALGOL string arrays — seven-backend matrix)

The matrix now executes a nested ALGOL procedure that forwards a captured,
non-unit-bound four-dimensional `string array` value formal to a sibling array formal.
The callee's lexical ordering, equality, and inequality checks prove the dynamic
`array<str>` handle, all four lower bounds, and all three row-major strides survived two
procedure boundaries on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

