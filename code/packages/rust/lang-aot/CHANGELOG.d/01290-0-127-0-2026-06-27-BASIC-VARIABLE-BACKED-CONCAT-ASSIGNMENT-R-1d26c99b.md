## 0.127.0 — 2026-06-27 — BASIC variable-backed concat assignment runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves a string expression assignment whose left operand is
a scalar string variable:

```basic
10 LET A$ = "O"
20 LET B$ = A$ + "K"
30 PRINT B$
40 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving E4 `str_concat` can store directly into another BASIC scalar string
slot before `print_str` consumes it.

