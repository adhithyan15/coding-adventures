## 0.220.4 - 2026-07-30 (ALGOL captured arrays — seven-backend matrix)

Test-only. The matrix now executes an ALGOL program where a proper procedure
mutates an enclosing `integer array values[4:5]` and the enclosing block reads
the result back as 42. It covers Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT,
locking in typed module-global array handles and the frontend's captured bounds.

