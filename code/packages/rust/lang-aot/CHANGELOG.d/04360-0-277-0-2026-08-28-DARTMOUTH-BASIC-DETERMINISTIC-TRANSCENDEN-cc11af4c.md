## 0.277.0 - 2026-08-28 (Dartmouth BASIC deterministic transcendental parity)

The unified language matrix now executes Dartmouth BASIC `SIN`, `COS`, `LOG`,
and `EXP` on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. One exact-value
program proves all four frontend spellings reach their existing shared AL8 IIR
operations and compose with the seven-backend BA7 real formatter.

Portable `RND` semantics and mixed numeric/string `DATA` remain independently
tracked because they require new contracts rather than transcendental wiring.

