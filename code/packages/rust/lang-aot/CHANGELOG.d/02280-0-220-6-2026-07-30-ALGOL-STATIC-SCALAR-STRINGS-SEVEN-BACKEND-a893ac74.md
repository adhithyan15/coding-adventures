## 0.220.6 - 2026-07-30 (ALGOL static scalar strings — seven-backend matrix)

The matrix now executes an ALGOL proper procedure that writes an enclosing
`string` and an integer procedure whose `own string memo` is initialized on
its first call and read on its second. The program exits with 3 across Native
AOT, LLVM, WASM, JVM, CLR, VM, and JIT, locking in typed persistent global
strings on every standard LANG target.

