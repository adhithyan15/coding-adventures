# Changelog

All notable changes to the grammar-tools package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-20

### Added
- **CLI validation tool** (`python -m grammar_tools validate <tokens> <grammar>`)
  - Catches typos in token/rule references, duplicate names, invalid regex
  - `validate-tokens` and `validate-grammar` for individual file validation
  - Warnings (unused tokens) don't fail; errors (undefined refs) do
  - 22 tests covering all commands and edge cases
- **Configurable escape processing** via `escapes:` directive in `.tokens` files
  - `escapes: none` disables string escape processing (CSS uses hex escapes)
  - Backward-compatible: files without `escapes:` use default behavior
- **Error token support** via `errors:` section in `.tokens` files
  - Error patterns are tried as fallback when no normal token matches
  - Tokens have `is_error` semantic (e.g., BAD_STRING, BAD_URL for CSS)
  - Backward-compatible: files without `errors:` behave as before

## [0.1.0] - 2026-03-20

### Added
- Initial package scaffolding with pyproject.toml, src layout, and test structure
- `.tokens` file format parser and validator (`token_grammar.py`)
- `.grammar` file format parser and validator (`parser_grammar.py`)
- Cross-validator for checking `.tokens` and `.grammar` files together
- Comprehensive test suite with >80% coverage
