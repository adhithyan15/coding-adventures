## 0.25.0 — 2026-06-09 — McCarthy **`LAMBDA`/`LABEL`/recursion** on the JVM — **JVM complete** (LANG77 / W5b, F7)

`compile_source_to_jvm` now runs McCarthy functions on a real `java`:
`((LAMBDA (X) X) 5)`→5, multi-arg lambdas, `(CAR ((LAMBDA (X) (CONS X X)) 7))`→7,
and a **recursive `LABEL`** walking a list to its atom→99. The win is in
`iir-builtin-lowering` 0.13.0 (lisp-`call` results typed `ref<any>` + reference
funnels) — the JVM backend already lowered `Object`-param/return methods +
`invokestatic`. **With this the JVM backend is McCarthy-complete (F1–F7)** — the
second managed backend done after WASM. New `tests/jvm_lambda.rs`.

