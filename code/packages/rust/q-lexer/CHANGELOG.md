# Changelog

## [0.1.0] - 2026-07-21

### Added

- Initial grammar-driven Rust Q tokenizer (MA11 §6, task MA-11b).
- Statically linked compiled token grammar (`code/grammars/q/q.tokens`),
  covering the historical-core subset fixed by MA-11a: dense numeric
  arrays, the primitive verb glyphs `+ - * % ! , # _ & | ~`, the six
  comparison glyphs `= < > <= >= <>`, the three adverbs (each/reduce/scan),
  `name:expr` assignment, parenthesised grouping, explicit `(a;b;c)` list
  literals, `{[x;y] ...}` function-literal delimiters, and `/`-to-end-of-line
  comments.
- Two whitespace-sensitive lexer disambiguations (MA11 §3 bullet 2), both
  genuinely novel for this crate family (every other array-language lexer
  in this repo treats whitespace as pure separator noise):
  - **Negative-literal vs. subtraction**: `2 -1` folds to the two-element
    strand `NUMBER(2) NUMBER(-1)`; `2 - 1` and `2-1` both stay
    `NUMBER(2) MINUS NUMBER(1)`. Implemented as a `GrammarLexer`
    post-tokenize hook (`fold_negative_number_literals`) that merges a
    `MINUS` immediately followed by a `NUMBER`, gated on whether the
    previously emitted token completes a noun glued directly against the
    `-` — not a hand-written lexer, a documented adjustment pass over the
    token stream the shared engine already produced.
  - **`/` comment marker vs. REDUCE adverb**: a `/` preceded by whitespace
    (or at line-start) opens a comment to end of line; glued directly to a
    preceding verb/noun with no space, it is REDUCE (`+/x`). Implemented as
    a `GrammarLexer` pre-tokenize hook (`strip_slash_comments`) that blanks
    comment text into spaces before the grammar ever runs — necessary
    because comment content is arbitrary text that need not lex
    successfully as Q code, so this cannot be a post-tokenize pass.
  - Both hooks reuse the exact `add_pre_tokenize`/`add_post_tokenize`
    extension points `scilab-lexer` already established for its own
    `'`-transpose-vs-string disambiguation — no changes to the shared
    `lexer`/`grammar-tools` crates were needed.
- No recursion-depth cap in this crate, by design: `q-lexer` performs no
  recursive descent at all (that begins with `q-parser`, MA-11c) — the same
  split every sibling `*-lexer`/`*-parser` pair in this repo already
  follows (`apl-lexer`/`j-lexer` have none either; `apl-parser`/`j-parser`
  do).
- `code/packages/rust/Cargo.toml` workspace registration alongside the
  other array-language lexer/parser/runtime/repl/to-semantic-ir crate
  groups.
