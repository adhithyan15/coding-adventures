### Added — Twig LANG-FULL E4 Derived Let Star String Parameter Evidence
- `twig-ir-compiler` 0.40.0 now proves sequential lexical `let*` string actuals
  derived from earlier string locals can seed conservative direct-call evidence
  for otherwise-unannotated string parameters. `(define (strlen x)
  (string-length x)) (let* ((a "HE") (b (string-append a "LLO"))) (strlen b))`
  stays on the typed E4 `str_concat` + `str_len` path without synthesizing
  refinement annotations.
- `lang-aot` adds the derived `let*` lexical-actual proof across native-AOT,
  LLVM, WASM, JVM, CLR, VM, and JIT.

