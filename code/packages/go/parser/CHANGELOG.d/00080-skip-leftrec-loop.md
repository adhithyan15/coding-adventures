## [0.4.3] - 2026-11-17

Performance only: about 2x on every benchmark grammar. No API change; trees,
positions, error messages, refusals, depth/frame accounting and memo-slot use
are identical.

### Changed
- **The left-recursion growth loop is skipped for rules that provably cannot
  left-recurse.** `parseRuleBody` used to re-run the body of every successful
  rule to try to grow the match, which for an ordinary rule re-boxed every
  token, built a second children slice and a memo-hit node per sub-rule, and
  threw it all away. `CompileGrammar` now computes `compiledRule.leftRec`
  (new `leftrec.go`): a nullable fixpoint, then a closure over left calls. The
  analysis over-approximates (optional, repetition and lookaheads are
  nullable; lookahead bodies and both halves of a separated repetition count
  as left calls; a token named `EOF` and an empty literal are nullable), so a
  rule is only skipped when it is provably safe.
- **Expected-sets use interned integer ids** (by string value, so
  de-duplication is unchanged); the "Expected A or B" text is built only when
  an error is reported. `matchToken` compares `TypeName` directly and calls
  `tokenTypeName` only for enum-only tokens.

### Tests
- `compileGrammar(g, forceLeftRec)` test knob; a differential over every
  `diffGrammars` entry, the benchmark grammars, extra EOF/separator grammars
  and 500 random grammars compares the analysis against forced-on for trees,
  positions, error text, slot count, depth, frames and furthest-failure set,
  under several `MaxDepth`/`MaxMemoSlots` settings.
- Analysis unit tests: direct, indirect, nullable prefix, lookahead, EOF,
  separated repetition, empty literal, and non-recursive controls.
- `BenchmarkParseStraceLike` on a committed copy of the strace grammar
  (`testdata/strace.grammar`).

### Review follow-ups
- The knob differential now also uses adversarial token streams (no trailing EOF, mid-stream EOF, empty, enum-only tokens), varies `StartRule`, uses 600 grammars x 60 inputs, and includes grammars with NEWLINE, EOF tokens and empty literals (seed 4242). Note: EOF/empty-literal nullability is conservative only (a zero-width match happens solely at the synthetic EOF, where nothing can grow), so those two mutants are behaviourally equivalent and are pinned by unit tests.
- `testdata/strace.grammar` is pinned byte-for-byte to the canonical file by a test; `expIDs` is dropped after compile; the strace grammar header comment no longer claims every rule consumes a token.
