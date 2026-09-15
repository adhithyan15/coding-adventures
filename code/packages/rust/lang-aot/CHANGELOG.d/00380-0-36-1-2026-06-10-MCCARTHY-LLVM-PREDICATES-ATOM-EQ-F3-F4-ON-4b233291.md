## 0.36.1 — 2026-06-10 — McCarthy → **LLVM predicates** ATOM/EQ (F3–F4) on a clang-built executable (LANG77 / W12b-2)

No `lang-aot` source change — the fix is in the shared `iir-builtin-lowering`
`lower_lisp_repr` (0.14.0), which `compile_source_to_llvm` already runs: a boolean
program result (a predicate) is now coerced with `lispy_truthy` (→ raw `0`/`1`)
instead of `lispy_unbox_int` (which gave `0` for *true*). New verify-by-running
test `tests/llvm_predicates.rs` (link `lispy_runtime.c` with clang, run):
`(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1, `(EQ 7 8)`→0. (`COND`, F5,
needs PHI-node merge of clause values across blocks — W12b-3.)

