## 0.220.51 - 2026-08-11 (ALGOL label shadowing — seven-backend matrix)

The matrix now proves lexical label shadowing on Native AOT, LLVM, WASM, JVM,
CLR, VM, and JIT. A nested `outer` label receives the nearest forward jump,
then scope exit restores the enclosing `outer`, producing 42 everywhere.

