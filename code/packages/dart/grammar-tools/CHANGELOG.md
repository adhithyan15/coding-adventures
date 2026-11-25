# Changelog

## Unreleased

- Fixed the Dart emitters rendering `$` bare in string literals (JSON
  escaping leaves it alone, but Dart reads it as interpolation), which
  made any grammar containing `$` (`css.tokens`) generate unparseable
  code. All string emissions now escape `$` as `\$`.
- Fixed the `.tokens` literal scanner skipping `\`-escapes when hunting
  the closing quote: literals are escape-blind by construction (Rust
  uses `rest.find('"')`, Go uses `strings.Index`), so `"\"` is the
  one-backslash pattern. The old scan rejected `apl.tokens` SCAN as
  unclosed.

## 0.1.0

- Initial Dart port of `grammar-tools`.
- Added `.tokens` parsing and validation.
- Added `.grammar` parsing and validation.
- Added cross-validation and source compilation helpers.
