## 0.16.0 — 2026-06-28 — Literal-backed string predicates (LANG-FULL AL4 on E4)

ALGOL 60 string comparisons now lower through the shared E4 string ops when both
operands are string literals or literal-backed scalar string variables:

```algol
begin string s; s := 'ALPHA';
  if (s = 'ALPHA' and s != 'OMEGA') and
     (s < 'BETA' and 'BETA' > s) then print('OK') else print('BAD')
end
```

`=`/`!=` use `str_eq` plus a typed zero comparison so the expression result is a
normal boolean. Ordering operators use `str_cmp` plus the corresponding typed
zero comparison. The slice remains intentionally fail-closed for unassigned
strings, captured/`own` strings, arrays, parameters, and dynamic string storage.

