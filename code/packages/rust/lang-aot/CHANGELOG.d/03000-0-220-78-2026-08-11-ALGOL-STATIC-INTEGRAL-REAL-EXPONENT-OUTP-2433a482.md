## 0.220.78 - 2026-08-11 (ALGOL static integral-real exponent output — seven backends)

The LANG matrix now proves positive and negative integral real-literal
exponents on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. Exact integrality
and the existing exponent cap select repeated arithmetic; fractional and
oversized real exponents remain outside the formatter-free path.

