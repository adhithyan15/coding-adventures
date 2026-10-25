---
category: Compiler / VM / language pipeline
---

# F10 transitions match a token's emitted type, so an alias target like NUMBER counts as defined

LANG84 phase 6a copied es2025's regex/division `transitions:` table into
es2015–es2024. To keep each copy valid, a script dropped any token name the
edition did not define. Its first version collected only rule names (the
left-hand side of `X = /re/`), so it dropped `NUMBER` and `STRING`. In the
ecmascript grammars those exist only as alias targets:

```
INT_NUMBER = /[0-9]+/ -> NUMBER
STRING_DQ  = /"…"/   -> STRING
```

Transitions match the token's *emitted* type, which is the alias. Without
`NUMBER` in the "value-producing" list, `8 / 2` would have lexed `/ 2…` as a
regex: the exact bug the table exists to prevent. The script printed its
dropped names, which is how it was caught.

**Do differently:**
- When filtering or validating transition, group or parser references
  against a `.tokens` file, the defined set is the rule names *plus* every
  `-> ALIAS` target.
- Print what the filter removed, and read it, before regenerating.
- Diff a real lex (`8/2/2` → `8` `/` `2` `/` `2`) in every edition you
  touched, not only the one you started from.

The same audit found es2025's own table missing `BIGINT` (`10n / 2n` started
a regex). That is fixed in the same phase.
