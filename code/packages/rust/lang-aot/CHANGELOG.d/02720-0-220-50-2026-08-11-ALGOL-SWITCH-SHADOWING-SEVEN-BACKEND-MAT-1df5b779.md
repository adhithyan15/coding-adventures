## 0.220.50 - 2026-08-11 (ALGOL switch shadowing — seven-backend matrix)

The matrix now proves lexical switch shadowing on Native AOT, LLVM, WASM, JVM,
CLR, VM, and JIT. A nested `s` selects its local target, then scope exit restores
the outer `s`, producing 42 on every standard backend.

