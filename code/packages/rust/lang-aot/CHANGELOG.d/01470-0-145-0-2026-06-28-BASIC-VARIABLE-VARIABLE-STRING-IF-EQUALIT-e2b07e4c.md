## 0.145.0 — 2026-06-28 — BASIC variable-variable string IF equality runs on all seven backends (LANG-FULL BA4)

The matrix now proves a BASIC string concat expression whose two operands are
both scalar string variables and whose result feeds the standard `=` branch:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 IF A$ + B$ = "OK" THEN 60
40 PRINT "BAD"
50 END
60 PRINT "OK"
70 END
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.30.0 lowers the expression to E4
`str_concat`, compares it with E4 `str_eq`, and branches on true equality.

