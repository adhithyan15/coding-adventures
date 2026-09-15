## 0.136.0 — 2026-06-27 — Twig local string equality branches run on all seven backends (LANG-FULL E4/TW4)

The Twig matrix now proves lexical string locals can feed E4 equality before
control flow:

```scheme
(let ((s "OK") (t "OK")) (if (string=? s t) 42 0))
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
The frontend emits local `str_const` slots, `str_eq`, and the existing branch
shape instead of dynamic `string=?`.

