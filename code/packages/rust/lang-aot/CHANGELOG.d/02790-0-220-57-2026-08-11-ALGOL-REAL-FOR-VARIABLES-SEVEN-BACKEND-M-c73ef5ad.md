## 0.220.57 - 2026-08-11 (ALGOL real for variables — seven-backend matrix)

The matrix now runs an ALGOL step/until loop controlled by a real array element
on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. Integer loop bounds widen to
the existing typed `f64` lowering.

