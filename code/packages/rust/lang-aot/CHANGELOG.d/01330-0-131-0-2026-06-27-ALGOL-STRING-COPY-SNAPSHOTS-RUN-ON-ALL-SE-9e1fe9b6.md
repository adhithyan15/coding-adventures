## 0.131.0 — 2026-06-27 — ALGOL string copy snapshots run on all seven backends (LANG-FULL AL4/E4)

The ALGOL matrix now proves scalar string copy snapshot semantics:

```algol
begin string s, t; s := 'OK'; t := s; s := 'NO'; print(t) end
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving a later source-slot `str_const` does not change the copied target slot.

