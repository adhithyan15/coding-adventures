---
category: Compiler / VM / language pipeline
---

# The Rust GrammarLexer never reads a .tokens errors: section, so malformed input must be a token that an E8 transition refuses

**What went wrong.** RS01 was written on the assumption that a `.tokens`
`errors:` entry makes the lexer fail on malformed input (reserved prefixes
`k#x`, a bad exponent `1e`, an unterminated string). Slice A4 ran the real
engine and found that `GrammarLexer` never reads `error_definitions`. The
section is parsed, validated and even compiled into `_grammar.rs` tables, and
then ignored: with `errors: BAD = /@+/`, the input `@@` lexes as two ordinary
`@` tokens. A pair that relied on it would have accepted every malformed
program the spec said it rejects, and `grammar-tools validate` would still have
passed.

**Fix.** Write the malformed construct as an ordinary token and refuse it with
a LANG84 E8 transition (`on BAD_TOKEN -> refuse`, or
`on RAW_IDENT="r#crate" -> refuse` to match one value). Give it a pattern that
consumes the characters that prove the error, order it before the token that
would otherwise match, and keep it out of the `.grammar`'s token set.

**Next time.**
- Before designing a lexical rule around a `.tokens` section, run a three-line
  grammar through `GrammarLexer` and see the section do something. A section
  that parses and validates is not a section the lexer uses.
- A refusal token must be ordered before the token it would otherwise lose to,
  and a `refuse` rule must precede any other rule for the same token (the first
  matching transition wins).
- Differential-test the result against the real compiler (here `rustc` 1.97.0,
  one process per case); it found two bugs in the refusal regexes themselves
  that no hand-written case had.
