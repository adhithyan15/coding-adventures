## 0.125.0 — 2026-06-27 — BASIC string expressions in PRINT run on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves `PRINT` can consume a temporary E4 string expression
result directly:

```basic
10 LET A$ = "O"
20 PRINT A$ + "K"
30 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving BA4 string-expression output without routing the concat through `LET`.

