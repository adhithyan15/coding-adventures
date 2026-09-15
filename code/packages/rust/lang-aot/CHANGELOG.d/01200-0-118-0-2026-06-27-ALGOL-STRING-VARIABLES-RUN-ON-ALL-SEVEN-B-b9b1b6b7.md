## 0.118.0 — 2026-06-27 — ALGOL string variables run on all seven backends (LANG-FULL AL4/E4)

The ALGOL 60 matrix now executes:

```algol
begin string s; s := 'HI'; print(s) end
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `HI`.

`algol-iir-compiler` 0.12.0 lowers literal assignments to scalar string slots as
direct E4 `str_const` producers and allows `print(s)` only for those
literal-backed slots. Dynamic string values, string copies, captured/`own`
strings, string arrays, and string parameters remain follow-up AL4 work.

