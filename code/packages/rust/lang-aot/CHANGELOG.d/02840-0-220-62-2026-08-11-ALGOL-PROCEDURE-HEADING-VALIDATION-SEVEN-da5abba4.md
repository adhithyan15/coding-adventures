## 0.220.62 - 2026-08-11 (ALGOL procedure heading validation — seven-backend matrix)

The matrix now runs a typed ALGOL procedure whose formals are partitioned
across integer and real specification groups on Native AOT, LLVM, WASM, JVM,
CLR, VM, and JIT. The frontend rejects malformed value and specification lists
before backend lowering.

