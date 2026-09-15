## 0.116.0 — 2026-06-27 — BASIC string variables run on all seven backends (LANG-FULL BA4/E4)

The Dartmouth BASIC matrix now executes:

```basic
10 LET A$ = "HI"
20 PRINT A$
30 END
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `HI`.

`coding-adventures-dartmouth-basic-lexer` 0.2.0 tokenizes `$`-suffixed names as
single `NAME` tokens, `coding-adventures-dartmouth-basic-parser` 0.2.0 accepts
`STRING` primaries, and `dartmouth-basic-iir-compiler` 0.15.0 lowers
literal-backed string variables to safe E4 `str_const` slots consumed by
`print_str`.

