# Changelog

All notable changes to the JSON parser package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_json_parser` now imports a pre-compiled `_grammar` module instead of reading and parsing the `json.grammar` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published PyPI package does not ship, so `pip install` + first use would raise `FileNotFoundError`.
- The pre-existing checked-in `src/json_parser/_grammar.py` predated a `grammar-tools` compiler update (missing the `# ruff: noqa` header the compiler now emits) and was never imported by `parser.py`, which always read `json.grammar` directly from disk. It has been regenerated fresh from the current grammar file and wired in.

## [0.1.0] - 2026-03-20

### Added
- Initial release of the JSON parser thin wrapper.
- `parse_json()` function for one-step parsing of JSON text into ASTs.
- `create_json_parser()` factory for creating configured `GrammarParser` instances.
- Full RFC 8259 grammar support: objects, arrays, strings, numbers, booleans, null.
- Produces generic `ASTNode` trees — the same type used for all grammar-driven languages.
