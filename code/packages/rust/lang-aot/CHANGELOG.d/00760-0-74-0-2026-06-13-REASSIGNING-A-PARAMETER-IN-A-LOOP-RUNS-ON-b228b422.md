## 0.74.0 — 2026-06-13 — reassigning a parameter in a loop runs on LLVM too (LANG-FULL — LLVM first-class)

`tests/lang_matrix.rs` gains an executed Nib program that accumulates into a
**function parameter** across a loop — `fn run(acc: u8) { for i in 0 .. 7 { acc = acc + 6 } return acc }`
→ exit 42 — asserted across native/LLVM/WASM/JVM/CLR/VM/JIT. This was the
limitation surfaced (and scoped out) in N2: the IIR-to-LLVM backend kept params in
SSA, so a reassigned param was silently dropped. Fixed in `iir-to-llvm` 0.10.0
(reassigned params are promoted to i64 stack slots, initialised from the incoming
argument). The other backends already handled it; this closes the LLVM gap so the
column is genuinely first-class. No `lang-aot/src` change.

