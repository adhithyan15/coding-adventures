## 0.76.0 — 2026-06-13 — Nib `&&` / `||` short-circuit executed on every backend (LANG-FULL N4)

`tests/lang_matrix.rs` gains three executed Nib programs proving short-circuit
`&&`/`||` across native/LLVM/WASM/JVM/CLR/VM/JIT: `1 == 2 && 84 / 0 == 0` → 7 and
`1 == 1 || 84 / 0 == 0` → 7 (the divide-by-zero right operand is positive proof it
was never evaluated — if it were, the program would trap), plus a `&&` true-path
program → 1. Backed by `nib-iir-compiler` 0.12.0 (`compile_short_circuit`). Frontend-
only — no backend change. No `lang-aot/src` change.

