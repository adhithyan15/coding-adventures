## 0.220.8 - 2026-07-31 (ALGOL integer-to-real promotion — seven-backend matrix)

The matrix now executes an ALGOL program that widens an integer into a real
array element, scalar, and real procedure parameter before comparing the array
element back to the original integer. It exits with 42 on Native AOT, LLVM,
WASM, JVM, CLR, VM, and JIT.

