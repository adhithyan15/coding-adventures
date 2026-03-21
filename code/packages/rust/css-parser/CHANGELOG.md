# Changelog

All notable changes to the `coding-adventures-css-parser` crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `create_css_parser(source)` — factory function that loads `css.grammar` and returns a configured `GrammarParser`.
- `parse_css(source)` — convenience function that parses CSS source and returns a `GrammarASTNode`.
- Loads grammar from `css.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering simple rules, multiple declarations, multiple rules, class/ID selectors, at-rules, empty stylesheets, descendant selectors, whitespace handling, and the factory function.
