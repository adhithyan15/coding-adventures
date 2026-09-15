## 0.212.0 - 2026-07-13 — E6d-8: Twig dynamic globals on the code-gen backends

A matrix cell proves a Twig **dynamic global** set+get roundtrip:

- `(define (f) g) (define g 42) (f)` → 42

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. A value global that is *forward-referenced*
(read inside `f` before its `define`) is emitted by the frontend as
`call_builtin "global_get"/"global_set"` — the dynamic `any`-typed global path
(a non-forward `define` gets a typed-local slot instead). The shared
`iir_builtin_lowering::lower_global_io` pass rewrites those to `global_load`/
`global_store` — the typed-global ops every backend accepts — **but only the native
`twig-aot` pipeline ran it**. The managed `lang-aot` pipelines (WASM/JVM/CLR/BEAM)
and the LLVM pipeline never called it, so a dynamic global reached the backend as
an unsupported `call_builtin` and failed to compile.

Fix: add `lower_global_io` (as pipeline step 0, before the value-model passes,
matching twig-aot) to `compile_source_to_wasm`, `compile_source_to_llvm_with_target`,
`compile_source_to_jvm_class`, `compile_source_to_cil_artifact`,
`compile_source_to_cil_text`, and `compile_source_to_beam`. No new backend code —
`global_load`/`global_store` were already supported. Run-verified locally on WASM
(in-process) + real dotnet CLR; native/LLVM/JVM via CI.

Follow-up (not in this change): a dynamic global whose value flows into *dynamic
arithmetic* (`(+ g 2)`) still traps, because the global slot stores a raw `i64`
while `lower_dynamic_arith` treats the `any`-typed `global_load` result as boxed and
inserts an `unbox`. Making that work needs the global slot widened to a boxed `any`
(the spec's "typed-global slots widened to any").

