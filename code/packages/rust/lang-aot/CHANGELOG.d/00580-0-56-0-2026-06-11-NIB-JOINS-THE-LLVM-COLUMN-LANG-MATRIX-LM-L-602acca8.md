## 0.56.0 — 2026-06-11 — Nib joins the LLVM column (LANG-MATRIX LM-L Nib)

`lang_matrix.rs` greens **Nib on LLVM** — the `Nib` program now lists `Llvm` among its
proven backends. The fix is in `nib-iir-compiler` 0.7.0 (it materialises integer types
to `i64` uniformly so the function signature and instruction bodies agree, which
`iir-to-llvm` requires). Verified by RUNNING: Nib `double(21)` → 42 on real `clang`,
and still → 42 on native AOT. The LLVM column is now green for Twig / Nib / Oct /
ALGOL 60; Brainfuck/BASIC pend the stdout-capturing LLVM I/O runner.

