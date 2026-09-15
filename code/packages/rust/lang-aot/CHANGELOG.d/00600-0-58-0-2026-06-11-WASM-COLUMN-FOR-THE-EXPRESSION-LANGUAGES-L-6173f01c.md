## 0.58.0 — 2026-06-11 — WASM column for the expression languages (LANG-MATRIX LM-W)

`lang_matrix.rs` gains a `Backend::Wasm` runner: source → wasm bytes (`iir-to-wasm`)
→ the in-process `wasm-runtime`, with `main`'s wasm result read as the exit value
(in-process, so no host gate — a WASM-tagged program that stops running fails the
floor test loudly). Verified by RUNNING — the expression languages run on WASM:
Twig→42, Oct→0, ALGOL `17 mod 5`→2. **14 proven matrix cells** (6 native + 5 LLVM +
3 WASM).

The other three WASM cells are scoped as follow-ups (gaps found by running): **Nib**
traps (`type mismatch: expected i64, got I32(21)`) — the const literal still emits
`i32` while the now-i64 param wants i64 (LLVM tolerated this via param-typed calls;
`iir-to-wasm` is stricter); **BASIC** needs the `PRINT` env import wired into the
runtime (`no body for function 0`); **Brainfuck** is deferred — `iir-to-wasm` lacks
the tape ops (`UnsupportedOp: alloc_bytes`), the same class as Brainfuck-on-LLVM.

