# coding-adventures-jsdoc-lexer

Grammar-driven lexer for the **interior** of a `/** ... */` JSDoc block
comment. Per [CLOC05](../../../specs/CLOC05-jsdoc-sub-pipeline.md).

The token grammar lives at
[`code/grammars/jsdoc/jsdoc.tokens`](../../../grammars/jsdoc/jsdoc.tokens) and
is compiled to native Rust at build time via
`grammar-tools compile-tokens`, embedded as `mod _grammar`.

## What's here (v1)

- `create_jsdoc_lexer(source)` → `GrammarLexer<'_>`
- `tokenize_jsdoc(source)` → `Vec<Token>` (panics on lex error)
- v1 token set: `AT_TAG`, type-expression brackets and punctuation,
  `NAME`, `NUMBER`, `STRING`, `NEWLINE`, plus a `DESCRIPTION_TEXT`
  catch-all for the prose after a tag's name path. See
  `jsdoc.tokens` for the full list.

## What's coming

- A comment-extractor stage that strips `/** */` markers and per-line
  `* ` continuation prefixes before invoking this lexer.
- More token categories as new tags ship (e.g. `@template`, `@typedef`).

## Inputs the lexer assumes are pre-processed

Callers must strip `/**` and `*/` markers and trim per-line `* `
continuations. The lexer has a `LEADING_STAR` skip as a safety net but
shouldn't normally see them.
