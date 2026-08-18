# Changelog

All notable changes to the JavaScript Parser package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_excel_parser` now imports a pre-compiled `_grammar` module instead of reading and parsing the `excel.grammar` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published PyPI package does not ship, so `pip install` + first use would raise `FileNotFoundError`.
- The pre-existing checked-in `src/excel_parser/_grammar.py` predated a `grammar-tools` compiler update (missing the `# ruff: noqa` header the compiler now emits) and was never imported by `parser.py`, which always read `excel.grammar` directly from disk. It has been regenerated fresh from the current grammar file and wired in.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the JavaScript parser package.
- `parse_excel()` function that parses JavaScript source code into generic `ASTNode` trees.
- `create_excel_parser()` factory function for creating a `GrammarParser` configured for JavaScript.
- Supports `var_declaration` (let/const/var), assignments, expression statements, and operator precedence.
- Comprehensive test suite with 80%+ coverage.
