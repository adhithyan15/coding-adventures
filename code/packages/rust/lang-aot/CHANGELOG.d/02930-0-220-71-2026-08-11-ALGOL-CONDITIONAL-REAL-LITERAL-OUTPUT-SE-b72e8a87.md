## 0.220.71 - 2026-08-11 (ALGOL conditional real-literal output — seven backends)

The LANG matrix now proves runtime selection between signed real-literal output
leaves on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. ALGOL lowers the
condition to typed control flow and each selected leaf to shared string output,
avoiding a runtime f64 formatter while computed or runtime leaves fail closed.

