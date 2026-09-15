## 0.214.0 - 2026-07-14 (E6d-7a-LLVM: closures on the LLVM column — all 5 code-gen backends)

With the iir-to-llvm dynamic-comparison-width fix (0.41.0) landed, the E6d-7a
closure pass (`lower_closures_to_heap`) is now wired into the LLVM pipeline
(`compile_source_to_llvm_with_target`, before `lower_heap_builtins_runtime`) — the
dispatcher's dynamic `=` index test previously mis-lowered to `icmp i1`. Closures
now run on **all 5 code-gen backends**: the two E6d-7 matrix cells
(`((lambda (x) (+ x 1)) 41)` and capturing `(((lambda (x) (lambda (y) (+ x y)))
40) 2)`, both -> 42) regain `Llvm`, and `e6d7a_wasm_closures` gains a run-verified
LLVM arm (5 tests: WASM 3, native 2... now + LLVM 2 = capture-free + capturing).
(Note: E6d-6 records/unions on LLVM remain a separate gap — they emit structural
`alloc`/`field_*` ops the LLVM backend doesn't support; closures avoid that by
riding the `cons`/`dyn_cons` runtime-call path.)



