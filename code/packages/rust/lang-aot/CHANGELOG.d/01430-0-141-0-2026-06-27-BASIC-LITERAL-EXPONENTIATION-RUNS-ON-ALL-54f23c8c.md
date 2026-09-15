## 0.141.0 — 2026-06-27 — BASIC literal exponentiation runs on all seven backends (LANG-FULL BA-^)

The matrix now proves the safe BASIC `^` subset:

```basic
10 PRINT 6 ^ 2 + 6
20 END
```

Expected stdout is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.26.0 lowers integer-valued literal exponents
through repeated `f64` multiplication, avoiding a backend math runtime.

