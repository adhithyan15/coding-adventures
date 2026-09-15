## 0.130.0 — 2026-06-27 — BASIC copied string equality runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves variable-to-variable string equality after a scalar
string copy:

```basic
10 LET A$ = "OK"
20 LET B$ = A$
30 IF B$ = A$ THEN 60
40 PRINT "BAD"
50 END
60 PRINT "OK"
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving copied string slots can feed E4 `str_eq` before BASIC line-control
branching.

