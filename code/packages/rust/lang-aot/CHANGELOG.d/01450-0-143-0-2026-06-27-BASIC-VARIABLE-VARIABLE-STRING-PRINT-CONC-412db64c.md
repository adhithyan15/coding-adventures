## 0.143.0 — 2026-06-27 — BASIC variable-variable string PRINT concat runs on all seven backends (LANG-FULL BA4)

The matrix now proves a BASIC string concat expression whose two operands are
both scalar string variables and whose result is consumed directly by `PRINT`:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 PRINT A$ + B$
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.28.0 lowers the expression to E4
`str_concat` and feeds the temporary result directly to `print_str`.

