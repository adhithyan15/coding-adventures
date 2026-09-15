## 0.201.0 - 2026-07-11 (E6d-2b: dynamic arithmetic on NativeAot + LLVM)

E6d-2b: the LLVM pipeline (`compile_source_to_llvm_with_target`) runs `lower_box_unbox_to_runtime_calls` after `lower_dyn_repr`, so a dynamic-arith program lowers to `__dyn_box_int`/`__dyn_unbox_int`. The E6d-2 matrix cell `(+ (car (cons 41 0)) 1)` -> 42 now covers all 5 code-gen backends [NativeAot, Llvm, Wasm, Jvm, Clr] (was Wasm/Jvm/Clr); `run_llvm` links the tagged-value runtime (dynval_runtime.c + twig_gc.c + twig_runtime.c) for `__dyn_*` programs. New integration test `e6d2b_dynamic_arith` proves native + LLVM = 42 by running.

