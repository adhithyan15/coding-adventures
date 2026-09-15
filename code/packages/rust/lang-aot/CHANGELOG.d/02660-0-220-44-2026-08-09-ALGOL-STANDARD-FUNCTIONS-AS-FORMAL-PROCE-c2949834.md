## 0.220.44 - 2026-08-09 (ALGOL standard functions as formal procedures — seven-backend matrix)

The matrix now forwards `abs` through nested ALGOL `procedure` formals on
Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The wrappers reuse the compiler's
inline standard-function lowering and return 42, proving this extension needs
no runtime function-pointer ABI.

