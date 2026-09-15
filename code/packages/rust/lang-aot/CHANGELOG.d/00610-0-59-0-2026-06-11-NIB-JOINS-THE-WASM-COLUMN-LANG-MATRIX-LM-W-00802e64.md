## 0.59.0 — 2026-06-11 — Nib joins the WASM column (LANG-MATRIX LM-W Nib)

`lang_matrix.rs` greens **Nib on WASM** (the `Nib` program now lists `Wasm`). The fix is
in `nib-iir-compiler` 0.8.0 (it finishes materializing Nib integer types to `i64` — the
const literals and the un-annotated-literal fallback, which previously stayed `u8` and
trapped the strict WASM backend). Verified by RUNNING: Nib `double(21)` → 42 on the
in-process `wasm-runtime`, still → 42 on LLVM and native. WASM column now green for
Twig / Nib / Oct / ALGOL 60; BASIC (print import) and Brainfuck (tape ops) remain.
**15 proven matrix cells.**

