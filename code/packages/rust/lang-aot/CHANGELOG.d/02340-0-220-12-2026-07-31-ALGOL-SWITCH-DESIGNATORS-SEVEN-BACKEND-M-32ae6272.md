## 0.220.12 - 2026-07-31 (ALGOL switch designators — seven-backend matrix)

The matrix now executes a switch whose selected element first evaluates a
boolean condition and then subscripts a second switch. It returns 42 on Native
AOT, LLVM, WASM, JVM, CLR, VM, and JIT, proving that switch-list designators are
evaluated at computed-goto time rather than flattened at declaration time.

