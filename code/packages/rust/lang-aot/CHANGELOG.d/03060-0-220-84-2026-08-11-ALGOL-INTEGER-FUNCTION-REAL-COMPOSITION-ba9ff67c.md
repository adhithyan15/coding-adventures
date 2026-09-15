## 0.220.84 - 2026-08-11 (ALGOL integer-function real composition — seven backends)

The LANG matrix now composes `sign` and `entier` results with literal-only real
arithmetic on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. Exact-range guards
keep the formatter-free widening deterministic.

