## 0.220.46 - 2026-08-09 (ALGOL value-mode formal procedures — seven-backend matrix)

The matrix now passes `square` through nested ALGOL `value procedure` formals
on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. Both direct specialisations
retain the static target and return 36 without a function-pointer ABI.

