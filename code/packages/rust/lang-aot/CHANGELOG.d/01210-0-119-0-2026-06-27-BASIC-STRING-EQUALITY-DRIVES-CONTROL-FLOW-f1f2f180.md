## 0.119.0 — 2026-06-27 — BASIC string equality drives control flow (LANG-FULL BA4/E4)

The Dartmouth BASIC matrix now executes:

```basic
10 LET A$ = "Y"
20 IF A$ = "Y" THEN 50
30 PRINT "BAD"
40 END
50 PRINT "OK"
60 END
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `OK`.

This proves the BA4 string-variable slice beyond printing: `dartmouth-basic-iir-compiler`
lowers `A$ = "Y"` in `IF` to shared E4 `str_eq`, and the resulting boolean feeds
the existing line-control branch machinery on every backend.

