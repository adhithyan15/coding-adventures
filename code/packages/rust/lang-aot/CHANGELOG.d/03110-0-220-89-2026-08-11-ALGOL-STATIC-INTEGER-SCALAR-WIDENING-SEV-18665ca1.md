## 0.220.89 - 2026-08-11 (ALGOL static integer scalar widening — seven backends)

The LANG matrix now copies an exact local integer snapshot, reassigns its
source, and widens the unchanged destination into mixed real output on Native
AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

