## 0.218.0 - 2026-07-14 (E6d closures on the generic VM/JIT — matrix columns)

The `lang_matrix` `run_vm` / `run_jit` helpers now apply the shared IIR-lowering
passes the code-gen pipelines already run for every language
(`lower_dynamic_for_generic_engine`: `lower_global_io` + `lower_closures_to_heap` +
`lower_heap_builtins`). Without them the generic register VM / JIT saw the raw
frontend `alloc_closure`/`cons`-builtin IIR they cannot dispatch; with them a
Twig closure lowers to its cons-chain object + synthesized dispatcher over ops the
VM now runs (`alloc`/`field_load`/`box`/dynamic `=`/`+` — added in vm-core 0.17/
0.18), so **closures run on all seven engines**. The two E6d-7 closure matrix
cells (`((lambda (x) (+ x 1)) 41)` and the capturing
`(((lambda (x) (lambda (y) (+ x y))) 40) 2)`) regain `Vm` + `Jit`.

Verified no regression: all 166 `Vm` and 166 `Jit` matrix cells still produce the
expected result under the added passes (they are no-ops on non-dynamic programs,
exactly as the code-gen pipelines rely on). `lang-aot` 0.217.0 → 0.218.0.

