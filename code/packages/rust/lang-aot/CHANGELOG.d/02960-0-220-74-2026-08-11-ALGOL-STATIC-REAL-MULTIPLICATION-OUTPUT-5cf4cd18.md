## 0.220.74 - 2026-08-11 (ALGOL static real multiplication output — seven backends)

The LANG matrix now proves finite literal-only real multiplication, including
normal operator precedence, on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.
The frontend emits the finite result through shared string output without
adding a runtime formatter; division and runtime operands remain unsupported.

