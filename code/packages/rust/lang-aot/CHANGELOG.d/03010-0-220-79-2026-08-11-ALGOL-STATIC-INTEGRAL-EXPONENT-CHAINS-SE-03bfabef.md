## 0.220.79 - 2026-08-11 (ALGOL static integral exponent chains — seven backends)

The LANG matrix now proves right-associated real and mixed integral-literal
exponent chains on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The frontend
checks every computed exponent against the fixed cap before deterministic
repeated multiplication and shared string output.

