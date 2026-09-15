## 0.220.10 - 2026-07-31 (ALGOL zero-argument procedures — seven-backend matrix)

The matrix now executes an ALGOL typed procedure invoked as `answer()` in a
value expression and a proper procedure invoked as `store()` in statement
position. The zero-argument IIR calls return 42 on Native AOT, LLVM, WASM, JVM,
CLR, VM, and JIT.

