# Changelog

All notable changes to the `coding-adventures-jsdoc-lexer` crate will be documented in this file.

## [0.1.0] - 2026-05-22

### Added
- New crate per CLOC05 Phase 1.
- `create_jsdoc_lexer(source) -> GrammarLexer<'_>` and `tokenize_jsdoc(source) -> Vec<Token>` factory + convenience functions.
- `_grammar.rs` auto-generated from `code/grammars/jsdoc/jsdoc.tokens` via `grammar-tools compile-tokens`. v1 token set: `AT_TAG`, type-expression brackets (`LBRACE`/`RBRACE`/`LBRACKET`/`RBRACKET`/`LPAREN`/`RPAREN`/`ANGLE_OPEN`/`ANGLE_CLOSE`), type-expression punctuation (`PIPE`/`AMP`/`COMMA`/`COLON`/`EQUALS`/`ELLIPSIS`/`QUESTION`/`BANG`/`STAR`/`ARROW`/`DOT`), `NEWLINE` as tag boundary, `NAME`, `NUMBER`, `STRING` (DQ + SQ both aliased), and `DESCRIPTION_TEXT` for the prose chunk after a tag's name path. Skip patterns for `HSPACE` and leading `* ` continuation. Three error tokens for unterminated strings and types.
- 8 tests covering: type tag tokenization, AT_TAG value content, dotted nominal types, nullable `?Foo` wrapper, variadic `...Foo` wrapper, `Foo[]` array suffix, whitespace insensitivity, factory path, empty input.

### Notes
- Callers must strip `/**` and `*/` markers and per-line `* ` continuations before invoking the lexer; the lexer's `LEADING_STAR` skip is a safety net only.
- Comment-extractor stage (CLOC05 §"jsdoc-comment-extractor") is a follow-up PR.
