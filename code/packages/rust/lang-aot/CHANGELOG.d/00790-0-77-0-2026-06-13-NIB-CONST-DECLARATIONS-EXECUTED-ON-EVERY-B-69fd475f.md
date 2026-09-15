## 0.77.0 — 2026-06-13 — Nib `const` declarations executed on every backend (LANG-FULL N5)

`tests/lang_matrix.rs` gains two executed Nib programs using module-scoped
`const`s — `const N: u8 = 42; … return N;` → 42 and `const A = 30; const B = 12;
… A + B` → 42 — across native/LLVM/WASM/JVM/CLR/VM/JIT. Backed by
`nib-iir-compiler` 0.13.0 (a const reference folds to its literal). Frontend-only;
no backend change. No `lang-aot/src` change.

