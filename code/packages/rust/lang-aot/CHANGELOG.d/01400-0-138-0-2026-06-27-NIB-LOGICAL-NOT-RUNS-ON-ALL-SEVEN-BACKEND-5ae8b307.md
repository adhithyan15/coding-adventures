## 0.138.0 — 2026-06-27 — Nib logical NOT runs on all seven backends (LANG-FULL N9)

The matrix now proves unary logical NOT:

```nib
fn main() -> u8 { if !(1 == 2) { return 42; } return 0; }
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`nib-type-checker` 0.5.0 types `!` as `bool`, and `nib-iir-compiler` 0.18.0
lowers it through the shared truthiness branch contract.

