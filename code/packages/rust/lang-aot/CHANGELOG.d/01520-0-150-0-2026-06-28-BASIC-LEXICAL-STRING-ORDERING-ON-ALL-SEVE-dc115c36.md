## 0.150.0 — 2026-06-28 — BASIC lexical string ordering on all seven backends (LANG-FULL BA4)

The matrix now proves Dartmouth BASIC `$` string ordering branches through the
shared E4 `str_cmp` op:

```basic
10 LET A$ = "ALPHA"
20 IF A$ < "BETA" THEN 40
30 END
40 IF "BETA" > A$ THEN 60
50 END
60 PRINT "OK"
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.31.0 lowers the string ordering result to
typed zero comparisons before reusing the standard BASIC line-control branch.

