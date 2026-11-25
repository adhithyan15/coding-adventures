---
category: Compiler / VM / language pipeline
---

# A grammar with no skip section still has the GrammarLexer silently skip spaces, tabs and CRs, so a whitespace-exact grammar needs a skip pattern that can never match

**What went wrong.** VER32 4.2 specified the Proof Interchange Format tokens
as "no skip section: spacing is part of the tokens" (every field token begins
with its one space). PIF-001 ran that file through the real `GrammarLexer`
and the first token containing a space failed to lex at all. `tokenize_standard`
in `code/packages/rust/lexer/src/grammar_lexer.rs` skips `' '`, `'\t'` and
`'\r'` unconditionally when `skip_patterns` is empty, before it tries any
definition. So "no skip section" means "skip the built-in blanks", not "skip
nothing": field tokens that start with a space never match, and doubled
spaces, tabs and CRs would be silently accepted.

**Fix.** Declare any skip pattern. That turns the built-in rule off. A pattern
that can never match makes the grammar truly whitespace-exact:

```text
skip:
  NEVER = /[^\x00-\x{10FFFF}]/
```

`code/grammars/pif/pif.tokens` does this and says why in a comment.

**Next time.** Before designing a lexical rule around "no skip", run a
three-line grammar through the real engine and feed it a doubled space and a
tab. A spec sketch of a `.tokens` file is a claim about the engine, not a
fact about it.
