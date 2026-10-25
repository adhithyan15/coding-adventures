---
category: Compiler / VM / language pipeline
---

# Claiming more editions for a subset exposes their lexer bugs to the compile path

LANG84 phase 6d made `es2016`–`es2025` claim the JavaScript numeric subset.
Until then the IIR path only ever lexed with es2020, so the other editions'
number patterns had never decided what a compiled program prints. The 6d
security review found the es2021+ patterns wrong in both directions:

- `[0-9]([0-9_]*[0-9])?` accepted invalid separators and the decoder stripped
  every `_`, so `console.log(0_17)` printed 15 (as legacy octal), `1__0`
  printed 10 and `0_8` printed 8. Node rejects all three as SyntaxErrors.
- `FLOAT_NUMBER` required a digit after `.`, so the valid `5.`, `5.e1` and
  `1.e3` were refused, some of them misleadingly as member accesses.

The corpus had one separator case (`1_000_000, 0.000_1`), which passed. The
claim was checked against node only on inputs that were already correct.

**Do this when a subset gains an edition:**
- Treat the new edition's lexer as untested on the compile path. Before
  claiming, run its number, string and identifier spellings, invalid ones
  included, against the host oracle (`node`, `python3`, `ruby`), and add a
  reject case for each form the oracle refuses.
- Read ECMA-262 (or the language reference) for the literal's grammar and
  encode its restrictions in the pattern. Here that means
  NumericLiteralSeparator: one `_` between two digits, none after a lone
  leading `0`, none in legacy octal (`017`) or NonOctalDecimal (`08`), and no
  fraction or exponent on a legacy octal.
- Keep a decoder from silently normalising what the lexer should have
  refused: refuse unexpected spellings instead of stripping characters.
