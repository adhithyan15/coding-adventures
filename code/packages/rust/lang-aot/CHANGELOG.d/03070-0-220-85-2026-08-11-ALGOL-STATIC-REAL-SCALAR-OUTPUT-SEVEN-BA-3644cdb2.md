## 0.220.85 - 2026-08-11 (ALGOL static real scalar output — seven backends)

The LANG matrix now assigns a finite static real expression to a local scalar
and prints its canonical value on Native AOT, LLVM, WASM, JVM, CLR, VM, and
JIT. The frontend invalidates this formatter-free shortcut at runtime control
flow, calls, captured state, or dynamic reassignment.

