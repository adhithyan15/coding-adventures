# Changelog

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
