## 0.148.0 — 2026-06-28 — Twig substring feeds string indexing on all seven backends (LANG-FULL E4)

The matrix now proves a typed Twig lexical string can be sliced and then byte
indexed:

```scheme
(let ((s "ABCDE")) (string-ref (substring s 1 4) 1))
```

Expected exit code is `67` (`C` in `BCD`) on native-AOT + LLVM + WASM + JVM +
CLR + VM + JIT. `twig-ir-compiler` 0.33.0 lowers `substring` to the shared
`str_slice` op, and the static backends either fold the derived literal slice
or map it to their managed string substring primitive.

