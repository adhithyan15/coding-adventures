## Unreleased — program names cannot shadow runtime symbols or compiler functions

- `compile_iir_to_llvm`, `compile_iir_to_wasm`, `compile_iir_to_jvm_class`, `compile_iir_to_cil_artifact`, `compile_iir_to_cil_text`, `compile_iir_to_beam` and `jit_lisp::run_mccarthy_on_jit` propagate `iir-builtin-lowering`'s new `HelperNameError` from `lower_closures_to_heap` / `lower_heap_builtins(_runtime)` as their backend error. A module that holds its own function named like a list or closure helper is refused instead of having `length` or closure calls routed to it.
- `lang_matrix`: new `program_names_cannot_shadow_runtime_symbols_or_compiler_functions`. It covers the three repros at three levels:
  - Twig refuses the source spellings.
  - The same shapes under innocent names run correctly on LLVM and native x86-64.
  - Renamed below the source check, the IIR is refused: a runtime-symbol name and a duplicate `__lambda_0` by the native link step (and the duplicate by `clang` on LLVM), and an impostor `__dyn_list_length` / `__dyn_call_closure` by the passes on both LLVM and native.
- `lower_dynamic_for_generic_engine` (lang_matrix, vm_gc_reclamation) and `e6d2b_dynamic_arith` unwrap the new `Result`.
- Security review follow-ups:
  - `compile_iir_to_llvm` runs `twig_aot::check_program_function_names` before any pass. The LLVM executable links the same C runtime and gc-core-capi, so a program `@__gc_alloc_kind`, `@__twig_*` or non-helper `@__dyn_*`, or two functions of one name, is refused as `LlvmBackendError`, as on the native column.
  - `program_names_cannot_shadow_runtime_symbols_or_compiler_functions` now also requires both columns to refuse `__twig_print_i64`, `__gc_alloc_kind`, `__gc_register_kind`, `__dyn_pair_first` and `__dyn_not_yet_defined`. The duplicate `__lambda_0` is now refused by the LLVM path itself rather than by clang.
  - New `twig_names_shaped_like_compiler_temporaries_keep_their_values` runs `(let ((_n2 40)) (+ _n2 2))`, `(define (f _n1) (+ _n1 2)) (f 40)` and `(define (g _else1) (if (= _else1 5) 42 0)) (g 5)`, expecting 42 on VM, JIT, LLVM, native, WASM and JVM (the call-arithmetic row skips the tagged-word columns). With the old `_n{k}` temporaries the VM computed 4.
