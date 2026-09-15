## 0.149.0 — 2026-06-28 — Twig lexical string ordering on all seven backends (LANG-FULL E4)

The matrix now proves nested `string<?` and `string>?` predicates via the shared
`str_cmp` op on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.

