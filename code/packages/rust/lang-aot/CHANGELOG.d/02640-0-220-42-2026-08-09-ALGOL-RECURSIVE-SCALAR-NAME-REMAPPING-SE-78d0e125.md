## 0.220.42 - 2026-08-09 (ALGOL recursive scalar name remapping — seven-backend matrix)

The matrix now runs a recursively remapped ALGOL scalar name pair on Native
AOT, LLVM, WASM, JVM, CLR, VM, and JIT. Each step swaps two writable name
formals, requiring the compiler to cycle between two specialised siblings
while retaining the original caller write targets; the base case returns 7.

