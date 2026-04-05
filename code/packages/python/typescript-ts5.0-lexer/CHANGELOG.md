# Changelog — TypeScript 5.0 (2023) Lexer

## 0.1.2 (2026-04-05)

- Add `BUILD_windows` for Windows CI compatibility (unquoted `.[dev]`, `uv run` instead of `.venv/bin/python`)

## 0.1.1 (2026-04-05)

### Fixed

- Moved `enum`, `async`, `await` from `reserved:`/`context_keywords:` to
  `keywords:` in ts5.0.tokens for consistent KEYWORD token emission

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 5.0 (2023) lexer
- Thin wrapper around `GrammarLexer` loading `ts5.0.tokens`
- Public API: `create_ts50_lexer(source)`, `tokenize_ts50(source)`
- Comprehensive test suite covering standard decorators, private class fields,
  static blocks, `using` keyword, `accessor`, `satisfies`, logical assignment
  operators, template literals, and TypeScript context keywords