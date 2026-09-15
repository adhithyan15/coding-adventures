### Added — Twig LANG-FULL E4 Static String Expression Parameter Evidence
- `twig-ir-compiler` 0.41.0 now proves conservative direct-call evidence for
  otherwise-unannotated string parameters can come from static string expression
  actuals, not only literals or named/lexical string values. `(define (strlen x)
  (string-length x)) (strlen (substring (string-append "HE" "LLO!") 0 5))`
  runs through typed E4 `str_concat` + `str_slice` + `str_len` without
  synthesizing refinement annotations.
- `lang-aot` adds the static-expression-actual proof across native-AOT, LLVM,
  WASM, JVM, CLR, VM, and JIT.

