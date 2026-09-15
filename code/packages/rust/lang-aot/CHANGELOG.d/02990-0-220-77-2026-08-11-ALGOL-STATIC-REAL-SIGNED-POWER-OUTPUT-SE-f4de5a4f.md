## 0.220.77 - 2026-08-11 (ALGOL static real signed-power output — seven backends)

The LANG matrix now proves positive and negative signed integer-literal powers
of real literals on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The frontend
uses capped repeated multiplication or division and emits only shared strings;
zero-base negative powers and broader exponent forms remain rejected.

