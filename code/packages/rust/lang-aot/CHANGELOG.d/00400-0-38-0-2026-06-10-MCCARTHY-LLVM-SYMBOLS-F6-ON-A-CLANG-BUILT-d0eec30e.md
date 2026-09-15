## 0.38.0 — 2026-06-10 — McCarthy → **LLVM symbols** (F6) on a clang-built executable (LANG77 / W13a)

No `lang-aot` source change — the work is in `iir-to-llvm` 0.7.0 (`symbol`→`i64`) and
`iir-builtin-lowering` 0.15.0 (symbol-result returned verbatim), which
`compile_source_to_llvm` already drives. New verify-by-running test
`tests/llvm_symbols.rs` (link `lispy_runtime.c` with clang, run):
`(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`→0, `(ATOM (QUOTE A))`→1,
symbol-in-`COND`→11, `(QUOTE A)`→its tagged word. LLVM is now F1–F6; only lambda
(F7, W13b) remains.

