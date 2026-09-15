### Added — Twig LANG-FULL E4 Named String Parameter Evidence
- `twig-ir-compiler` 0.38.0 now lets conservative `main`-level direct-call
  evidence for otherwise-unannotated string parameters use non-escaping
  top-level string value actuals, so `(define s "HELLO") (define (strlen x)
  (string-length x)) (strlen s)` runs through the typed E4 `str_len` path.
- The inference pass stays source-order and escape-analysis aware: captured,
  shadowed, conflicting, unobserved, and closure-derived values remain on the
  dynamic path and do not synthesize refinement annotations.
- `lang-aot` adds the named-actual proof across native-AOT, LLVM, WASM, JVM,
  CLR, VM, and JIT.

