## 0.220.27 - 2026-08-02 (ALGOL runtime procedure strings in arrays — seven-backend matrix)

The matrix now stores branch-selected `string procedure` results in `array<str>`,
reads them back for lexical ordering, and prints the selected element on Native
AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

