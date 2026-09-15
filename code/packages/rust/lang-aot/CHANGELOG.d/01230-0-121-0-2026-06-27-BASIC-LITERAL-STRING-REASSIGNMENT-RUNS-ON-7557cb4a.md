## 0.121.0 — 2026-06-27 — BASIC literal string reassignment runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves that a scalar string variable can be assigned more
than once from literals and that the latest literal is the one printed:

```basic
10 LET A$ = "NO"
20 LET A$ = "OK"
30 PRINT A$
40 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
This stays inside the literal-backed E4 subset by re-emitting `str_const` into
the same safe backend-facing slot; non-literal copies, string arrays, string
`INPUT`, and dynamic byte-string storage remain follow-up BA4/E4 work.

