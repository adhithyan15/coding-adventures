## 0.220.67 - 2026-08-11 (ALGOL integer output — seven-backend matrix)

An ALGOL `output(n + 2)` matrix cell now prints `42` on Native AOT, LLVM, WASM,
JVM, CLR, VM, and JIT through the existing shared `print_i64` builtin. No
ALGOL-specific backend hook or procedure ABI change is required.

