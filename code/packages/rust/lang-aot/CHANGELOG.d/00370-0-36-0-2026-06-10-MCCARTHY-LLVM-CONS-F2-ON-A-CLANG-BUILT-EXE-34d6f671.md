## 0.36.0 — 2026-06-10 — McCarthy → **LLVM cons** (F2) on a clang-built executable (LANG77 / W12b-1)

`compile_source_to_llvm` now runs the **native tagged-word lisp pipeline** — the
SAME passes the native AOT path runs, NOT the managed structural pass:
`lower_heap_builtins_runtime` (cons/car/cdr → `call_builtin "lispy_*"`) →
`intern_symbols` → `lower_lisp_repr` (boxes int literals to tagged words, inserts the
final `lispy_unbox_int` so the result is a plain `i64`). `iir-to-llvm` (0.5.0) lowers
each `lispy_*` to `call @__twig_lispy_*`. A pure-scalar program never enters those
passes (then `concretize_scalar_any_for_llvm` handles `any`→`i64` as before).

**Verified by RUNNING** (`tests/llvm_cons.rs`): emit host-triple IR, **link
`twig-aot/runtime/lispy_runtime.c`** with `clang` (`-x ir <ours> -x none <runtime.c>`),
run the native executable — exit code = result: `(CAR (CONS 7 9))`→7,
`(CDR (CONS 7 9))`→9, `(CAR (CDR (CONS 1 (CONS 2 3))))`→2, scalar `42`→42 (no
regression). Predicates (pair?/equal?/not, COND — F3–F5) are W12b-2 (their
tagged-boolean result needs its own handling).

