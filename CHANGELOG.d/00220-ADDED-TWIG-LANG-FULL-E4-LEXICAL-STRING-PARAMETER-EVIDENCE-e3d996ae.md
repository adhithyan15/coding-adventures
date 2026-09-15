### Added — Twig LANG-FULL E4 Lexical String Parameter Evidence
- `twig-ir-compiler` 0.39.0 now lets conservative `main`-level direct-call
  evidence for otherwise-unannotated string parameters use lexical `let`/`let*`
  string actuals, so `(define (strlen x) (string-length x)) (let ((s "HELLO"))
  (strlen s))` runs through the typed E4 `str_len` path.
- Lexical evidence is scoped: dynamic shadows and non-string local bindings
  still block inference and remain on the dynamic path without synthesizing
  refinement annotations.
- `lang-aot` adds the lexical-actual proof across native-AOT, LLVM, WASM, JVM,
  CLR, VM, and JIT.

