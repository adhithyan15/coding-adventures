## 0.124.0 — 2026-06-27 — BASIC scalar string copy runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves literal-backed string-to-string assignment:

```basic
10 LET A$ = "OK"
20 LET B$ = A$
30 PRINT B$
40 END
```

The frontend lowers the copy as E4 `str_concat` with an empty suffix, so the
row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT
without adding a dedicated string-copy opcode.

