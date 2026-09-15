## 0.220.73 - 2026-08-11 (ALGOL static additive real output — seven backends)

The LANG matrix now proves finite literal-only real addition and subtraction on
Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The frontend evaluates the
bounded expression at compile time and emits only shared string output; runtime
operands and non-additive arithmetic remain unsupported.

