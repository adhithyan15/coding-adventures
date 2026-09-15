## 0.146.0 — 2026-06-28 — Twig string concat feeds string indexing on all seven backends (LANG-FULL E4)

The matrix now proves a typed Twig lexical string append whose result feeds
byte indexing:

```scheme
(let ((a "AB") (b "CDE") (i 3)) (string-ref (string-append a b) i))
```

Expected exit code is `68` (`D` in `ABCDE`) on native-AOT + LLVM + WASM + JVM +
CLR + VM + JIT. `twig-ir-compiler` 0.31.0 lowers the local strings to E4
`str_const`, appends them with E4 `str_concat`, and consumes the temporary
directly with E4 `str_index`.

