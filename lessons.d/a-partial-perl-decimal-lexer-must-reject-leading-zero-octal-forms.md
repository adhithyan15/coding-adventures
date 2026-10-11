---
category: Perl
---

# A partial Perl decimal lexer must reject leading-zero octal forms

A versioned Perl arithmetic-print subset used `[0-9]+` for integers. That
accepted `08` and `09` as decimal input even though the release tokenizer
interprets a leading zero as octal and rejects those digits. A parser-only
smoke test did not expose the mismatch.

When the subset intentionally excludes octal literals, tokenize only `0` or
`[1-9][0-9]*`. Probe `08`, `09`, and `012` against every release pair that
claims plain-decimal integers. Check historical `toke.c` before carrying a
numeric rule forward between versions.
