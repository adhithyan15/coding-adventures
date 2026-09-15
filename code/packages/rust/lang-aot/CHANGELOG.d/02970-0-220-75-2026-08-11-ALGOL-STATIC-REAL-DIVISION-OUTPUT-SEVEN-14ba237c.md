## 0.220.75 - 2026-08-11 (ALGOL static real division output — seven backends)

The LANG matrix now proves finite literal-only real division and multiplicative
left associativity on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. Zero
divisors and non-finite quotients remain explicit compile errors, preserving the
formatter-free runtime boundary.

