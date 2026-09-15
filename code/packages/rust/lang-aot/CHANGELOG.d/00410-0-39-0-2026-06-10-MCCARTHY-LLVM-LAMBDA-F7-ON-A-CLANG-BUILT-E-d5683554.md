## 0.39.0 — 2026-06-10 — McCarthy → **LLVM lambda** (F7) on a clang-built executable — **LLVM COMPLETE F1–F7** (LANG77 / W13b)

No `lang-aot` source change — the work is in `iir-builtin-lowering` 0.16.0 (lambda
arg boxing + polymorphic result coercion), `twig-aot`'s `lispy_runtime.c` 0.14.0
(the `__twig_lispy_to_exit_code` runtime switch) and `iir-to-llvm` 0.8.0 (declaring
it), all of which `compile_source_to_llvm` already drives. New verify-by-running
test `tests/llvm_lambda.rs` (link `lispy_runtime.c` with clang, run):
`((LAMBDA (X) X) 5)`→5, `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7,
`((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, `((LAMBDA (X) (ATOM X)) 7)`→1,
lambda-with-`COND`-body→100/200. **LLVM is now McCarthy-complete (F1–F7)** — the
sixth backend to finish, after VM/WASM/JVM/CLR/BEAM.

