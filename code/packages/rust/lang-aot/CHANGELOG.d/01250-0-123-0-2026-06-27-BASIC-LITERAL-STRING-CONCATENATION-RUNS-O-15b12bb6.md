## 0.123.0 — 2026-06-27 — BASIC literal string concatenation runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves a source-level string expression over E4
`str_concat`:

```basic
10 LET A$ = "O" + "K"
20 PRINT A$
30 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
The frontend materializes both literals, writes the concatenation into the safe
`A$` string slot, and then prints that slot. Non-literal copies and dynamic
byte-string storage remain follow-up BA4/E4 work. The LLVM leg is backed by
`iir-to-llvm` 0.21.0, which binds derived literal concat constants so
`str_concat` can feed `print_str`.

