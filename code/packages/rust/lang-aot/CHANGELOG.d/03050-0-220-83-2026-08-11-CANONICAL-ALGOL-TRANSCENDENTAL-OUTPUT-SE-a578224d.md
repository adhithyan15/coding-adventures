## 0.220.83 - 2026-08-11 (canonical ALGOL transcendental output — seven backends)

The LANG matrix now prints exact zero/one identities for `sin`, `cos`, `ln`,
`exp`, and `arctan` through shared static strings on Native AOT, LLVM, WASM,
JVM, CLR, VM, and JIT, without runtime math or f64 formatting.

