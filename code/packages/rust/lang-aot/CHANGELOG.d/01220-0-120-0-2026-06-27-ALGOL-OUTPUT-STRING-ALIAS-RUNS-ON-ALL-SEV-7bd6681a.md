## 0.120.0 — 2026-06-27 — ALGOL `output` string alias runs on all seven backends (LANG-FULL AL4/E4)

The ALGOL 60 matrix now executes:

```algol
begin string s; s := 'OK'; output(s) end
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `OK`.

This is the same literal-backed scalar string slot as the `print(s)` proof, but
through the implementation-defined `output` spelling. It closes the current AL4
output alias proof without widening into dynamic string values.

