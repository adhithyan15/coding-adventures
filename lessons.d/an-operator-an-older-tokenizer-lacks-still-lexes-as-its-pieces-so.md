---
category: Testing & coverage
---

# An operator an older tokenizer lacks still lexes as its pieces, so test operator sets by token kind, not by lexing success

**What went wrong.** LANG84 phase 7a gave Python 3.0a1 its own
`python3.0a1.tokens`, without `:=` (PEP 572, 3.8) or `@=` (PEP 465, 3.5).
The probe meant to prove that was `assert!(!lexes("3.0a1", "a := 1"))`. It
failed, and the test was what was wrong. `:` and `=` are both tokens in
3.0a1, so `a := 1` lexes fine there, as `NAME COLON EQUALS INT`. CPython
3.0a1's tokenizer splits it the same way. "Does not lex" was never the
difference between the releases. The difference is "lexes as one token
versus two".

**Fix.** The probe now compares token kinds: `COLON, EQUALS` in 3.0a1 and
`WALRUS` in 3.11.17, and the same for `AT, EQUALS` versus `AT_EQUALS`. Only
an operator built from a character the old release has no token for, such
as a lone `!` before PEP 701 (3.12), shows up as a lexing failure.

**Next time.** When a per-release `.tokens` file drops a multi-character
operator, check the kinds of the tokens the source becomes. A missing
compound operator almost always decomposes into operators that do exist,
and a lex-success probe passes or fails for the wrong reason. Compare
against the real tokenizer's split (CPython's `tokenize`, Ripper, Acorn)
where one exists.
