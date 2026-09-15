## 0.220.24 - 2026-08-01 (ALGOL runtime integer exponents — seven-backend matrix)

The matrix now executes runtime positive and negative integer exponents from an integer
base. The compiler widens both operands to the shared `f64_pow` operation, so `2^3` and
the reciprocal `2^-1` run on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

