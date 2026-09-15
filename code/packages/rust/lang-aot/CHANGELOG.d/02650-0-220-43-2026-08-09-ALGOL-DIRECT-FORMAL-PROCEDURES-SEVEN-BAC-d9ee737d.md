## 0.220.43 - 2026-08-09 (ALGOL direct formal procedures — seven-backend matrix)

The matrix now passes `square` through an ALGOL `procedure` formal and forwards
that formal through a nested wrapper on Native AOT, LLVM, WASM, JVM, CLR, VM,
and JIT. Both specialised wrappers call the original declared target directly,
proving the compiler needs no new runtime function-pointer ABI for this slice.

