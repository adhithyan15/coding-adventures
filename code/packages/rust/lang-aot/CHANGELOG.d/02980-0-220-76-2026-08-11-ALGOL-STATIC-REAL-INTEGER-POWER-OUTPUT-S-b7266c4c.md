## 0.220.76 - 2026-08-11 (ALGOL static real integer-power output — seven backends)

The LANG matrix now proves finite real literal bases with capped nonnegative
integer-literal exponent chains on Native AOT, LLVM, WASM, JVM, CLR, VM, and
JIT. Frontend repeated multiplication preserves right associativity without a
runtime formatter or platform-dependent compile-time `pow` call.

