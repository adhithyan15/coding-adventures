## 0.15.0 — 2026-06-08 — McCarthy → WebAssembly, **cons** (LANG77 / L3b-3a-3c)

`compile_source_to_wasm` now compiles McCarthy **cons** programs — not just
scalars — to a runnable WasmGC module. The pipeline gains the structural
representation pass between the heap lowering and the scalar concretizer:

```
lower_heap_builtins            cons/car/cdr → alloc/field_store/field_load
lower_lisp_repr_structural     box atoms → i31ref, unbox the entry result   ← new
concretize_scalar_any_for_wasm any → i64 for the remaining pure-scalar fns
```

The two representation passes partition the module's functions (heap-using vs
pure-scalar), so every value ends up concretely typed. **`(CAR (CONS 7 9))`
emits a `.wasm` that runs to `7`** on the in-repo `wasm-runtime`; `CDR` and
nested cons work too. The previous "cons is cleanly unsupported" test is
replaced by these end-to-end runs. Scalar McCarthy and Twig programs are
unaffected (regression-tested).

