## 0.220.37 - 2026-08-09 (ALGOL call-by-name forwarding — seven-backend matrix)

The matrix now executes a nested ALGOL direct call-by-name forwarding case on
Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The inner formal intentionally
shares the outer name, proving the compiler preserves the original caller
binding rather than accidentally capturing the callee-local spelling.

