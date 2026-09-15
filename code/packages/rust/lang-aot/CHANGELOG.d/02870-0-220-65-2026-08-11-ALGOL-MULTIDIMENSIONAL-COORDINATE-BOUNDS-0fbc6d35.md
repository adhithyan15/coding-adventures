## 0.220.65 - 2026-08-11 (ALGOL multidimensional coordinate bounds — seven-backend matrix)

The matrix now proves that an out-of-range second coordinate cannot alias the
next valid row after row-major flattening. Native AOT, LLVM, WASM, JVM, CLR,
VM, and JIT all fail closed through their existing array bounds traps.

