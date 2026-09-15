## 0.220.36 - 2026-08-08 (ALGOL direct call-by-name — seven-backend matrix)

The matrix now runs a Jensen-style ALGOL sum with a name-bound loop variable
and `i * i` term on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The result
proves writes reach the caller variable and each formal read re-evaluates the
caller expression through the shared typed IIR path.

