## 0.220.7 - 2026-07-31 (ALGOL array value parameters — seven-backend matrix)

The matrix now executes an ALGOL integer procedure with a one-dimensional
`integer array` value parameter. The callee writes and reads `values[4:5]`
through its parameter and returns 42; a proper procedure forwards the captured
descriptor, covering global handle/lower-bound reload. The program runs on
Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

The JVM scalar-narrowing pass now narrows `array<i64>` formal descriptors to
`array<i32>` together with their allocation/access instructions. This keeps the
method signature's `int[]` aligned with `iaload`/`iastore`, preventing a real
JVM verifier failure.

