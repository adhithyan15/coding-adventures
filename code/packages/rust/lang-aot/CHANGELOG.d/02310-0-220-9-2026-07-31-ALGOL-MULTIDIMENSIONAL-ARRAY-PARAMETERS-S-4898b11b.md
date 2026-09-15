## 0.220.9 - 2026-07-31 (ALGOL multidimensional array parameters — seven-backend matrix)

The matrix now executes an ALGOL procedure with a two-dimensional `integer
array` value parameter whose actual is captured by a forwarding procedure. Its
nonzero lower bounds force the shared call ABI to preserve the typed handle,
both lower bounds, and the outer row-major stride. The callee writes two cells
and returns 42 on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

