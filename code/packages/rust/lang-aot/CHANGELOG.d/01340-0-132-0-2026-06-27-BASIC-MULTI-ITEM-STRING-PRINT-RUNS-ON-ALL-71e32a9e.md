## 0.132.0 — 2026-06-27 — BASIC multi-item string PRINT runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves two scalar string slots in one `PRINT` statement:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 PRINT A$; B$
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving ordered repeated `print_str` calls for `;`-separated string items.

