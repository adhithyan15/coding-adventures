## 0.73.0 — 2026-06-13 — Nib `for` loops executed on every backend (LANG-FULL N2)

`tests/lang_matrix.rs` gains two executed Nib `for`-loop programs — a sum loop
`for i in 1 .. 6 { s = s + i }` → exit 15 (uses the loop variable) and a nested
loop (3 × 2) → exit 6 — asserted across native/LLVM/WASM/JVM/CLR/VM/JIT. Backed
by `nib-iir-compiler` 0.10.0 (lowers `for` to the canonical counter loop). The
matrix battery now exercises real cross-backend loop control flow with counter +
accumulator reassignment, not just straight-line arithmetic. No `lang-aot/src`
change.

