## 0.151.0 — 2026-06-28 — ALGOL string predicates on all seven backends (LANG-FULL AL4)

The matrix now proves ALGOL 60 literal-backed scalar string equality and
ordering branches through the shared E4 `str_eq` / `str_cmp` ops:

```algol
begin string s; s := 'ALPHA';
  if (s = 'ALPHA' and s != 'OMEGA') and
     (s < 'BETA' and 'BETA' > s) then print('OK') else print('BAD')
end
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`algol-iir-compiler` 0.16.0 lowers the string predicate results to typed zero
comparisons before reusing the standard ALGOL conditional branch.

