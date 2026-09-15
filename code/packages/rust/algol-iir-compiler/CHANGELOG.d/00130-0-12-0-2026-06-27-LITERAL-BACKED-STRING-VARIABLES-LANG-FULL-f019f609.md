## 0.12.0 — 2026-06-27 — Literal-backed string variables (LANG-FULL AL4 on E4)

ALGOL 60 now supports the first scalar string-variable foothold:

```algol
begin string s; s := 'HI'; print(s) end
```

The `string` declaration records a `str` slot, assignment from a string literal
emits `str_const` directly to that slot, and `print(s)` is accepted only when
the slot is known to be literal-backed. That shape is important for the static
backends: WASM/LLVM/native/JVM/CLR can all consume the same direct E4 producer
metadata already proven by literal output.

This is still intentionally fail-closed. Unassigned string variables,
string-to-string copies, captured/`own` string globals, string procedures, and
string arrays remain outside the dynamic string model and produce explicit
errors.

