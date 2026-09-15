## 0.220.81 - 2026-08-11 (composed ALGOL static real functions — seven backends)

The LANG matrix now nests formatter-free `abs` and exact `sqrt` calls and
combines their values with literal-only addition, subtraction, and
multiplication on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

