# Changelog — TypeScript 5.8 (2025) Lexer

## 0.1.2 (2026-04-05)

- Add `BUILD_windows` for Windows CI compatibility (unquoted `.[dev]`, `uv run` instead of `.venv/bin/python`)

## 0.1.1 (2026-04-05)

### Fixed

- Moved `enum`, `async`, `await` from `reserved:`/`context_keywords:` to
  `keywords:` in ts5.8.tokens so they emit as `KEYWORD` tokens (matches
  ES2025 semantics where `async`/`await` are hard keywords)

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 5.8 (2025) lexer
- Thin wrapper around `GrammarLexer` loading `ts5.8.tokens`
- Public API: `create_ts58_lexer(source)`, `tokenize_ts58(source)`
- Comprehensive test suite covering `using`/`await using` resource management,
  import attributes, HASHBANG token, standard decorators, private class fields,
  TypeScript context keywords, ES2025 keyword set, logical assignment operators,
  and template literals