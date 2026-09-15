## 0.220.70 - 2026-08-11 (ALGOL signed real-literal output — seven backends)

An ALGOL `output(-4.25, +2.5)` matrix cell now prints `-4.25+2.5` on Native
AOT, LLVM, WASM, JVM, CLR, VM, and JIT. Exact unary signs remain attached to
the parser-validated literal spelling; binary expressions do not enter this
bounded source-spelling path.

