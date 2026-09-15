## 0.220.86 - 2026-08-11 (ALGOL static real scalar copies — seven backends)

The LANG matrix now copies a tracked static real local, reassigns its source,
and prints the unchanged destination snapshot on Native AOT, LLVM, WASM, JVM,
CLR, VM, and JIT.

