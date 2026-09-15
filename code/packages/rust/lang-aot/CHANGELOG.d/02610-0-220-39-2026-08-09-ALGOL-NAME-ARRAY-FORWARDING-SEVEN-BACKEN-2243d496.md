## 0.220.39 - 2026-08-09 (ALGOL name-array forwarding — seven-backend matrix)

The matrix now executes a nested two-dimensional ALGOL name-array forwarding
case on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. Both the outer and inner
formals deliberately share a spelling; their writes and reads prove the handle,
non-unit lower bounds, and outer stride stay attached to the original caller
array descriptor.

