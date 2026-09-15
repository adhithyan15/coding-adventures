## 0.220.66 - 2026-08-11 (ALGOL fail-closed array extents — seven-backend matrix)

The matrix now evaluates a reversed ALGOL array bound pair from run-time scalar
variables and proves that Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT all trap
before allocation. Compiler tests also guard extent and row-major product
overflow without changing the array descriptor ABI.

