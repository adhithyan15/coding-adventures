# Changelog — TypeScript 1.0 (April 2014) Lexer

## 0.1.2 (2026-04-05)

- Add `BUILD_windows` for Windows CI compatibility (unquoted `.[dev]`, `uv run` instead of `.venv/bin/python`)

## 0.1.1 (2026-04-05)

### Fixed

- Corrected test names: `EXCLAMATION` → `BANG`, `FAT_ARROW` → `ARROW`,
  `QUESTION_MARK` → `QUESTION` to match actual token names in ts1.0.tokens
- Fixed `test_enum_is_name` → `test_enum_is_keyword`: `enum` is a hard keyword
  in TypeScript 1.0 (promoted from ES5 reserved word)

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 1.0 (April 2014) lexer
- Thin wrapper around `GrammarLexer` loading `ts1.0.tokens`
- Public API: `create_ts10_lexer(source)`, `tokenize_ts10(source)`
- Comprehensive test suite covering type annotations, context keywords,
  generics, decorators, union types, arrow types, non-null assertions,
  interfaces, optional parameters, and ES5 compatibility
- PEP 561 `py.typed` marker for type checker support