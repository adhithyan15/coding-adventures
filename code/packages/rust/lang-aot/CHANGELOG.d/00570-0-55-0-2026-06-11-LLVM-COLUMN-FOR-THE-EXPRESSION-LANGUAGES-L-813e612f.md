## 0.55.0 — 2026-06-11 — LLVM column for the expression languages (LANG-MATRIX Phase L)

`tests/lang_matrix.rs` refactored into a `Backend`-keyed grid: every `Prog` lists the
backends a slice has **proven** run it, and `matrix_every_proven_cell_agrees` runs
each proven cell on its real toolchain and asserts the known result (a
`proven_columns_do_not_silently_skip` floor catches a tool-present cell that stops
running). Added a `clang`-gated **LLVM runner** (source → textual `.ll` via
`iir-to-llvm` → real `clang` → run). Verified by RUNNING — the expression languages
run on real LLVM: Twig→42, Oct→0, ALGOL `17 mod 5`→2 (no C runtime needed; they link a
bare `.ll`).

Two LLVM cells deferred to focused follow-up slices, with the gaps recorded by
running: **Nib** hits a real `iir-to-llvm` bug (`u8` operands mis-widened to `i64` —
`'%x' defined with type 'i8' but expected 'i64'` on `add`; native AOT runs Nib fine,
so the IIR is sound and only LLVM mishandles the narrow type), and **Brainfuck/BASIC**
need the stdout-capturing LLVM runner that links the C I/O runtime.

