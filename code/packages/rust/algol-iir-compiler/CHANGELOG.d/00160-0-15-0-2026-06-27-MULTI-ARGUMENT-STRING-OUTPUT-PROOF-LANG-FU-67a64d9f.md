## 0.15.0 — 2026-06-27 — Multi-argument string output proof (LANG-FULL AL4 on E4)

ALGOL 60 `output` now has an explicit proof for multiple literal-backed scalar
string variables in one statement:

```algol
begin string s, t; s := 'O'; t := 'K'; output(s, t) end
```

The compiler preserves actual order by emitting two `print_str` calls over the
literal-backed E4 string slots. The matrix observes `OK` on every LANG backend
without adding an ALGOL-specific output hook or dynamic procedure call.

