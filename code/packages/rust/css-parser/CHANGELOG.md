# Changelog

All notable changes to the `coding-adventures-css-parser` crate will be documented in this file.

## [0.1.1] - 2026-07-18

### Fixed
- **Security hardening**: `create_css_parser` never called `GrammarParser::with_max_depth`, leaving every caller (including this crate's own `parse_css`) exposed to a native-stack-overflow DoS from adversarial deeply-nested input. Added a `MAX_RULE_DEPTH = 170` cap, derived from independently measuring `css.grammar`'s five distinct self-referential recursion shapes (nested qualified-rule blocks, nested `@media` at-rules, nested `@supports`/paren conditions, nested `calc()` calls, nested `:not()` pseudo-class args) — binary search over candidate `with_max_depth` values against a 5000-deep adversarial input per shape. Binding floor: nested `@media` at 247/248 (safe/crash). Cap sits ~31% below that. 3 new depth-guard regression tests.

## [0.1.0] - 2026-03-21

### Added
- `create_css_parser(source)` — factory function that loads `css.grammar` and returns a configured `GrammarParser`.
- `parse_css(source)` — convenience function that parses CSS source and returns a `GrammarASTNode`.
- Loads grammar from `css.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering simple rules, multiple declarations, multiple rules, class/ID selectors, at-rules, empty stylesheets, descendant selectors, whitespace handling, and the factory function.
