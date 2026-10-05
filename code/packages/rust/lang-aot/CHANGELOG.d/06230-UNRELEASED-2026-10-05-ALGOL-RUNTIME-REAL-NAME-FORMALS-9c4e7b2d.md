## 0.440.0 - 2026-10-05 - ALGOL runtime-real name formals

The seven-backend ALGOL matrix now executes a specialised read-only real name
formal whose non-assignable actual is a runtime real procedure result. Native
AOT, LLVM, WASM, JVM, CLR, VM, and JIT all print the value through the shared
portable formatter.
