## 0.128.0 — 2026-06-27 — ALGOL scalar string copy runs on all seven backends (LANG-FULL AL4/E4)

The ALGOL matrix now proves literal-backed scalar string copies:

```algol
begin string s, t; s := 'OK'; t := s; print(t) end
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving the AL4 frontend can copy a literal-backed string slot through E4
`str_concat` with an empty suffix before `print_str` consumes the target.

