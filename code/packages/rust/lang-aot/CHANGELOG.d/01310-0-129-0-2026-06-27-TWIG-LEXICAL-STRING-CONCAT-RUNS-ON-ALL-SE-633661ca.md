## 0.129.0 — 2026-06-27 — Twig lexical string concat runs on all seven backends (LANG-FULL E4/TW4)

The Twig matrix now proves lexical string locals can feed E4 concat:

```scheme
(let ((a "AB") (b "CDE")) (string-length (string-append a b)))
```

The row expects exit `5` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving local non-literal string operands can flow through `str_concat` and
`str_len` without falling back to dynamic builtins.

