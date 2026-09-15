## 0.126.0 — 2026-06-27 — BASIC string expressions in IF run on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves string expressions can feed line-control
comparisons:

```basic
10 LET A$ = "O"
20 IF A$ + "K" = "OK" THEN 50
30 PRINT "BAD"
40 END
50 PRINT "OK"
60 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving E4 `str_concat` can feed `str_eq` before the existing `jmp_if_true`
branch.

