## 0.13.0 — 2026-06-27 — Literal-backed scalar string copies (LANG-FULL AL4 on E4)

ALGOL 60 scalar string assignment now accepts a literal-backed string variable
RHS:

```algol
begin string s, t; s := 'OK'; t := s; print(t) end
```

The compiler lowers `t := s` as E4 `str_concat t, s, ""`, materializing the
empty suffix as `str_const`. The target is marked literal-backed so `print(t)`
can consume it through `print_str`. Unassigned string sources still fail closed.

