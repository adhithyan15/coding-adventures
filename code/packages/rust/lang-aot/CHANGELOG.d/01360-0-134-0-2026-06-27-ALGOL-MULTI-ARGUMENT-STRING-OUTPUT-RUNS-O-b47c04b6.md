## 0.134.0 — 2026-06-27 — ALGOL multi-argument string output runs on all seven backends (LANG-FULL AL4/E4)

The ALGOL matrix now proves multiple literal-backed scalar string actuals in one
`output` statement:

```algol
begin string s, t; s := 'O'; t := 'K'; output(s, t) end
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
The compiler emits two ordered E4 `print_str` calls and avoids dynamic procedure
dispatch, keeping the proof inside the current AL4 immutable scalar string
foothold.

