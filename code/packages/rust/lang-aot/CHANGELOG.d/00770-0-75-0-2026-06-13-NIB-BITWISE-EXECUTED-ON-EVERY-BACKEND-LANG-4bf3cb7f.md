## 0.75.0 — 2026-06-13 — Nib bitwise `& | ^` executed on every backend (LANG-FULL N3)

`tests/lang_matrix.rs` gains three executed Nib programs — `12 & 10` → 8,
`12 | 3` → 15, `6 ^ 5` → 3 — asserted across native/LLVM/WASM/JVM/CLR/VM/JIT.
Backed by `nib-iir-compiler` 0.11.0 (lowers `& | ^` to the shared IIR
`and`/`or`/`xor`). The executed test surfaced a CLR backend gap — the textual
`.il` path didn't emit the bitwise opcodes — fixed in `iir-to-cil-bytecode`
0.19.0. No `lang-aot/src` change.

