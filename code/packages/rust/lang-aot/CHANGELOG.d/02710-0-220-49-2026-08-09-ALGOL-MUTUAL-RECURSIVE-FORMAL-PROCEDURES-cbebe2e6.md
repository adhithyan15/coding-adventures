## 0.220.49 - 2026-08-09 (ALGOL mutual recursive formal procedures — seven-backend matrix)

The matrix now mutually forwards `twice` through ALGOL `procedure` formals on
Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The specialised `even` and `odd`
siblings close the recursion graph before a direct base call returns 42, without
a runtime procedure descriptor.

