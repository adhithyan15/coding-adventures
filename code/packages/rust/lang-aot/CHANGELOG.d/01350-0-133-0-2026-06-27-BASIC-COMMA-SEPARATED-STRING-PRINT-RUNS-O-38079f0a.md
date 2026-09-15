## 0.133.0 — 2026-06-27 — BASIC comma-separated string PRINT runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves BA2 comma separators compose with BA4/E4 string
items:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 PRINT A$, B$
```

Expected stdout is `O K` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
The compiler emits the existing comma separator as `putchar(' ')` between two
ordered `print_str` calls, so string items remain on the shared E4 output path
and do not route through numeric formatting helpers.

