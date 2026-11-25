# Changelog

## Unreleased

### Changed
- Retired the hand-written recursive-descent parser: `parse()` now runs
  the shared `parser::grammar_parser::GrammarParser` over the checked-in
  `src/_grammar.rs` (compiled from `code/grammars/lisp/lisp.grammar`)
  and lowers the CST onto the unchanged public `SExpr`/`AtomKind`/
  `ParseError` API. All 25 existing tests pass unmodified, as do the
  `lisp-compiler` (48) and `lisp-vm` (84) consumer suites.
- Added `lisp_parser_grammar_spec()` exposing the compiled grammar for
  tooling, plus a drift test pinning `_grammar.rs` to `lisp.grammar`.

### Fixed
- A leading dot (`(. b)`) is now a parse error per the grammar
  (`list_body` requires an element before `DOT`); the hand reader
  accepted it as an empty-headed pair.
- Added a `MAX_NESTING_DEPTH` (48) pre-scan: over-deep input now fails
  with a catchable error instead of overflowing the native stack
  (previously an uncatchable abort in both parse and drop paths).

## 0.1.0 -- 2026-03-21

### Added
- Initial implementation of the Lisp parser, ported from the Python `lisp-parser` package.
- `SExpr` enum with variants: `Atom`, `List`, `DottedPair`, `Quoted`.
- `AtomKind` enum: `Number`, `Symbol`, `String`.
- `parse()` function that converts source text to a vector of S-expressions.
- `parse_tokens()` function for parsing a pre-tokenized token stream.
- Support for dotted pair notation: `(a . b)`.
- Support for quoted forms: `'x`, `'(1 2 3)`.
- Custom `ParseError` type with descriptive messages.
- Comprehensive test suite covering atoms, lists, quotes, dotted pairs, and complex expressions.
