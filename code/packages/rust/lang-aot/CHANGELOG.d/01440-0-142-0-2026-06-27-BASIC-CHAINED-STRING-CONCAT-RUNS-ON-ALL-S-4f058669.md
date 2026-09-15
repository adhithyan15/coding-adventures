## 0.142.0 — 2026-06-27 — BASIC chained string concat runs on all seven backends (LANG-FULL BA4)

The matrix now proves a left-associative BASIC string concat chain:

```basic
10 LET A$ = "A"
20 LET B$ = A$ + "B" + "C"
30 PRINT B$
```

Expected stdout is `ABC` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.27.0 lowers the chain to repeated E4
`str_concat` instructions and stores the final value in the target scalar
string slot.

