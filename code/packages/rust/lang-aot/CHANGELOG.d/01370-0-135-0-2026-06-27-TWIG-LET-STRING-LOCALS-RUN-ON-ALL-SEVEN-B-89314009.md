## 0.135.0 — 2026-06-27 — Twig `let*` string locals run on all seven backends (LANG-FULL E4/TW4)

The Twig matrix now proves the sequential lexical binding form can feed E4
string ops:

```scheme
(let* ((s "HELLO")) (string-length s))
```

Expected exit code is `5` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
This backs the existing `twig-ir-compiler` unit proof with an all-backend run:
`let*` materializes a typed `str_const` local slot, then `str_len` consumes it
without falling back to dynamic `string-length`.

