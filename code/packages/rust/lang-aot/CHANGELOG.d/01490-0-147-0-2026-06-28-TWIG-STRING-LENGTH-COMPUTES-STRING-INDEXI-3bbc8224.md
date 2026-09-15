## 0.147.0 — 2026-06-28 — Twig string length computes string indexing on all seven backends (LANG-FULL E4)

The matrix now proves a typed Twig lexical string length can feed integer
arithmetic and then byte indexing:

```scheme
(let ((s "ABCDE")) (string-ref s (- (string-length s) 1)))
```

Expected exit code is `69` (`E` in `ABCDE`) on native-AOT + LLVM + WASM + JVM +
CLR + VM + JIT. `twig-ir-compiler` 0.32.0 lowers the local string to E4
`str_const`, computes the index with E4 `str_len` plus typed `sub`, and consumes
that computed register with E4 `str_index`; `twig-aot` 0.20.0 folds the same
metadata chain before direct native lowering, and `iir-to-llvm` 0.22.0 does the
same for the LLVM column.

