## Unreleased — program names cannot shadow runtime symbols or compiler functions

- `compile_iir_to_llvm`, `compile_iir_to_wasm`, `compile_iir_to_jvm_class`, `compile_iir_to_cil_artifact`, `compile_iir_to_cil_text`, `compile_iir_to_beam` and `jit_lisp::run_mccarthy_on_jit` propagate `iir-builtin-lowering`'s new `HelperNameError` from `lower_closures_to_heap` / `lower_heap_builtins(_runtime)` as their backend error. A module that holds its own function named like a list or closure helper is refused instead of having `length` or closure calls routed to it.
- `lang_matrix`: new `program_names_cannot_shadow_runtime_symbols_or_compiler_functions`. It covers the three repros at three levels:
  - Twig refuses the source spellings.
  - The same shapes under innocent names run correctly on LLVM and native x86-64.
  - Renamed below the source check, the IIR is refused: a runtime-symbol name and a duplicate `__lambda_0` by the native link step (and the duplicate by `clang` on LLVM), and an impostor `__dyn_list_length` / `__dyn_call_closure` by the passes on both LLVM and native.
- `lower_dynamic_for_generic_engine` (lang_matrix, vm_gc_reclamation) and `e6d2b_dynamic_arith` unwrap the new `Result`.
