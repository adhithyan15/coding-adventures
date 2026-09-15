## 0.140.0 — 2026-06-27 — Oct logical NOT runs on all seven backends (LANG-FULL O-!)

The matrix now proves unary logical NOT for Oct:

```oct
fn main() { if !(1 == 2) { out(1, 42); } else { out(1, 0); } }
```

Expected stdout is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`oct-iir-compiler` 0.9.0 lowers `!` through the shared truthiness branch
contract, materialising a clean 0/1 bool instead of reusing bitwise `not`.

