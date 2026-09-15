## 0.14.0 — 2026-06-27 — Scalar string copy snapshot proof (LANG-FULL AL4 on E4)

ALGOL 60 scalar string copy now has an explicit snapshot proof:

```algol
begin string s, t; s := 'OK'; t := s; s := 'NO'; print(t) end
```

The compiler still lowers `t := s` through E4 `str_concat` with an empty
suffix. The new regression proves the copied target slot remains independently
printable after the source slot is rematerialized with a later `str_const`.

