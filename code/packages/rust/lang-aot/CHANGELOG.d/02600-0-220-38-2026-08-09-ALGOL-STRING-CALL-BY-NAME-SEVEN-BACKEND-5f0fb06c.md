## 0.220.38 - 2026-08-09 (ALGOL string call-by-name — seven-backend matrix)

The matrix now executes a nested ALGOL direct string call-by-name forwarding
case on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The inner formal shadows
the outer spelling, writes `OK` through the original caller binding, then reads
it back through runtime string equality before returning 42.

