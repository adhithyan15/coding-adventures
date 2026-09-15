## 0.115.0 — 2026-06-27 — Lexical Twig string locals run on all seven backends (LANG-FULL E4)

The Twig matrix now executes `(let ((s "ABC") (i 2)) (string-ref s i))` on
**native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, returning exit code `67`
everywhere.

`twig-ir-compiler` 0.28.0 materializes lexical `let`/`let*` string literal
bindings as typed `str_const` registers and lets known local string/index
registers feed the E4 string-op lowering. `twig-aot` 0.19.0 carries literal
metadata through the local integer `mov` so the native column folds the same
`str_index` as the other static backends. This closes the local string-slot proof
for the current literal/named-value E4 foothold without claiming
captured/reassigned strings or broader dynamic byte-string values.

