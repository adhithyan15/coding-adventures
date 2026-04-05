# Changelog — TypeScript 4.0 (2020) Lexer

## 0.1.2 (2026-04-05)

- Add `BUILD_windows` for Windows CI compatibility (unquoted `.[dev]`, `uv run` instead of `.venv/bin/python`)

## 0.1.1 (2026-04-05)

### Fixed

- Added ES2021 logical assignment operators: `AND_AND_EQUALS = "&&="`,
  `OR_OR_EQUALS = "||="`, `NULLISH_EQUALS = "??="` to ts4.0.tokens
- Corrected token ordering: `NULLISH_EQUALS` placed before `NULLISH_COALESCE`
  so `??=` is matched before `??` (first-match-wins)
- Moved `enum`, `async`, `await` to `keywords:` in ts4.0.tokens

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 4.0 (2020) lexer
- Thin wrapper around `GrammarLexer` loading `ts4.0.tokens`
- Public API: `create_ts40_lexer(source)`, `tokenize_ts40(source)`
- Comprehensive test suite covering keywords, type annotations, operators
- Tests for TS 4.0-specific features: short-circuit assignment (`&&=`, `||=`, `??=`),
  labeled tuple elements, variadic tuple types, template literal types