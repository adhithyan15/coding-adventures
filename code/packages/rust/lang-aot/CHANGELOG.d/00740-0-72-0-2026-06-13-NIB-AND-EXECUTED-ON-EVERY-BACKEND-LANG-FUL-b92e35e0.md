## 0.72.0 — 2026-06-13 — Nib `*` and `/` executed on every backend (LANG-FULL N1)

First slice of the LANG-FULL campaign (full implementations of every matrix
language, each feature **verified by RUNNING** on every backend, not just
validated/encoded). `tests/lang_matrix.rs` gains two executed Nib programs —
`fn main() -> u8 { return 6 * 7; }` → exit 42 and `… 84 / 2; }` → exit 42 —
asserted across native/LLVM/WASM/JVM/CLR/VM/JIT. Backed by `nib-iir-compiler`
0.9.0 (lowers `*`/`/` to the shared IIR `mul`/`div`). No `lang-aot/src` change;
the matrix battery now has more than one executed program per language.

