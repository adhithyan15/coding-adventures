## 0.137.0 — 2026-06-27 — Nib static globals run on all seven backends (LANG-FULL N8)

The matrix now proves module-scoped mutable Nib statics through the shared E6
global substrate:

```nib
static counter: u8 = 40;
fn bump(step: u8) -> u8 { counter = counter + step; return counter; }
fn main() -> u8 { let a: u8 = bump(1); let b: u8 = bump(1); return counter; }
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`nib-type-checker` 0.4.0 makes static names visible across functions, while
`nib-iir-compiler` 0.17.0 seeds `main` with `global_store` and lowers reads/writes
to `global_load`/`global_store`.

