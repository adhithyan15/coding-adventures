### Added — Twig LANG-FULL E4 Multi-Parameter String Evidence
- `twig-ir-compiler` 0.42.0 now proves one conservative direct call can infer
  multiple otherwise-unannotated string parameters at once. `(define (same a b)
  (if (string=? a b) 42 0)) (same "OK" (string-append "O" "K"))` lowers the
  function body through typed E4 `str_eq` without synthesizing refinement
  annotations.
- `lang-aot` adds the multi-parameter string-equality proof across native-AOT,
  LLVM, WASM, JVM, CLR, VM, and JIT.

