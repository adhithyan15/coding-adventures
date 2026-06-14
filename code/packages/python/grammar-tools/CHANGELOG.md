# Changelog

All notable changes to the grammar-tools package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - F10 declarative lexer mode transitions

### Added
- `ModeTransition` and `TransitionAction` dataclasses, plus `start_mode` and
  `transitions` fields on `TokenGrammar`, porting the F10 declarative
  lexer-mode transition table (`F10-declarative-lexer-modes.md`) from the Rust
  reference. A `.tokens` grammar can declare context-sensitive lexing
  (regex-vs-division, template substitutions) as data rather than a callback.
- `.tokens` parsing: a `start_mode:` directive and a `transitions:` section
  (`on TOKENS [in MODE] -> ACTION, ...`; actions `set-mode`/`push`/`pop`/
  `enable-skip`/`disable-skip`; `KEYWORD="value"` guard). Validation rejects
  undefined target/guard modes and caps rule count (`MAX_TRANSITIONS`).
- `compile_token_grammar` emits the two new fields (the dataclass reprs are
  valid constructor source); the generated import line carries the new types.

### Notes
- Backward compatible: a grammar with no `transitions:`/`start_mode:` parses to
  empty defaults. Interpreting the table in the Python `lexer` package is a
  follow-up (mirrors the Rust grammar-tools → lexer split).

## [0.3.0] - 2026-03-26

### Added
- **Grammar compiler** (`compile_token_grammar`, `compile_parser_grammar`) in `grammar_tools.compiler`
  - Converts a parsed `TokenGrammar` or `ParserGrammar` into Python source code
  - Generated code embeds all grammar data as native Python data structures (no runtime I/O)
  - Round-trip fidelity: compiling then loading recreates an equivalent grammar object
  - Header comment in each generated file explains origin and how to regenerate
  - 33 new tests covering all grammar element types, edge cases, and round-trips

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
