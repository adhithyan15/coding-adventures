## 0.139.0 — 2026-06-27 — Nib const/static expression folding runs on all seven backends (LANG-FULL N10)

The matrix now proves a folded Nib const expression feeding a folded static
initializer:

```nib
const BASE: u8 = 6 * 7;
static counter: u8 = BASE + 0;
fn main() -> u8 { return counter; }
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`nib-type-checker` 0.6.0 types the initializer expressions, and
`nib-iir-compiler` 0.19.0 folds them before runtime code is emitted.

