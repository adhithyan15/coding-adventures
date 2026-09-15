## 0.220.82 - 2026-08-11 (conditional ALGOL static real functions — seven backends)

The LANG matrix now lets a runtime boolean select between formatter-free
`abs` and exact-`sqrt` expressions on Native AOT, LLVM, WASM, JVM, CLR, VM, and
JIT. The selected branch prints through shared strings without runtime f64
formatting.

