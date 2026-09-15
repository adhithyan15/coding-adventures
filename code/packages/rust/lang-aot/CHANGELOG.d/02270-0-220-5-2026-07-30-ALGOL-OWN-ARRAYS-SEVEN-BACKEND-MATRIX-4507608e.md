## 0.220.5 - 2026-07-30 (ALGOL own arrays — seven-backend matrix)

Test-only. The matrix now executes an ALGOL procedure containing `own integer
array memo[4:5]`. Three calls update and read the same `memo[4]` cell, producing
1 + 2 + 3 = 6 across Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. This locks
in first-call-only allocation and persistent declared-bound metadata.

