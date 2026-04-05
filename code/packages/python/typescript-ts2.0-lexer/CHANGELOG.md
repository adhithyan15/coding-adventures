# Changelog — TypeScript 2.0 (September 2016) Lexer

## 0.1.2 (2026-04-05)

- Add `BUILD_windows` for Windows CI compatibility (unquoted `.[dev]`, `uv run` instead of `.venv/bin/python`)

## 0.1.1 (2026-04-05)

### Fixed

- Corrected test names: `FAT_ARROW` → `ARROW`, `EXCLAMATION` → `BANG`
- Updated `test_let_is_name` and `test_const_is_name`: `let` and `const` are
  hard keywords in TypeScript 2.0 (ES2015 baseline), not context keywords
- Renamed tests to `test_let_is_keyword` and `test_const_is_keyword`

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 2.0 (September 2016) lexer
- Thin wrapper around `GrammarLexer` loading `ts2.0.tokens`
- Public API: `create_ts20_lexer(source)`, `tokenize_ts20(source)`
- Comprehensive test suite covering the `never` type, template literals,
  ES2015 `let`/`const`, arrow functions, ES2015 classes, ES2015 modules,
  destructuring, default parameters, and TS 1.0/ES5 compatibility
- PEP 561 `py.typed` marker for type checker support