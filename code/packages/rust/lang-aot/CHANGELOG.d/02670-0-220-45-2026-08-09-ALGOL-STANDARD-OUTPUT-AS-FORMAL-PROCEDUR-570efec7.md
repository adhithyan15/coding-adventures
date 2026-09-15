## 0.220.45 - 2026-08-09 (ALGOL standard output as formal procedures — seven-backend matrix)

The matrix now forwards the implementation-defined `print` procedure through
nested ALGOL `procedure` formals on Native AOT, LLVM, WASM, JVM, CLR, VM, and
JIT. The nested wrapper reuses `print_str` lowering and produces `FORMAL`,
proving this statement-only target needs no runtime procedure ABI.

