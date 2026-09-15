## 0.117.0 — 2026-06-27 — ALGOL literal string output runs on all seven backends (LANG-FULL AL4/E4)

The ALGOL 60 matrix now executes:

```algol
begin print('HI') end
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `HI`.

`algol-iir-compiler` 0.11.0 recognises undeclared statement-position
`print`/`output` calls as standard output procedures and lowers string literal
actuals to shared E4 `str_const` + `print_str`. Full ALGOL string variables,
string arrays, and non-literal string expressions remain follow-up AL4 work.

