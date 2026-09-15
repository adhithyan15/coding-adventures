## 0.220.72 - 2026-08-11 (ALGOL parenthesized real-literal output — seven backends)

The LANG matrix now proves that exact parentheses around signed direct real
literals preserve their source spelling on Native AOT, LLVM, WASM, JVM, CLR,
VM, and JIT. Parenthesized arithmetic remains outside the bounded path.

