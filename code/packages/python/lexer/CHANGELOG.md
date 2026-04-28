# Changelog

All notable changes to the lexer package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2026-03-31

### Fixed
- **STRING case preservation in case-insensitive mode**: When a grammar uses
  `# @case_insensitive true`, the lexer lowercases the source for pattern
  matching. Previously this also lowercased STRING token values — `'Ada'`
  would tokenize as `STRING("ada")` instead of `STRING("Ada")`. The fix
  preserves the original source separately (`_original_source`) and uses
  it when extracting string literal bodies, so case in string content is
  always preserved regardless of the grammar's case-sensitivity setting.

## [0.3.0] - 2026-03-21

### Added
- **Pattern group support**: The lexer now supports named pattern groups from
  `.tokens` files (`group NAME:` sections). When groups are defined, the lexer
  uses a group stack to determine which patterns to try at each position.
- **On-token callback hooks**: Register a callback via `set_on_token()` that
  fires on every token match. The callback receives a `LexerContext` for
  controlling group transitions, token emission, and skip processing.
- **LexerContext API**: New class providing `push_group()`, `pop_group()`,
  `emit()`, `suppress()`, `peek()`, `peek_str()`, `set_skip_enabled()`,
  `active_group()`, and `group_stack_depth()` methods for callback use.
- Exported `LexerContext` from the package's public API.

## [0.2.0] - 2026-03-20

### Added
- **Configurable escape processing**: When `escape_mode == "none"` in the
  token grammar, string tokens have quotes stripped but escape sequences
  are preserved raw (needed for CSS hex escapes like `\26`)
- **Error token fallback**: When no normal token matches, the lexer tries
  error patterns from the `errors:` section before raising `LexerError`.
  Error tokens allow graceful degradation for malformed input.

## [0.1.0] - 2026-03-20

### Added
- Initial package scaffolding with pyproject.toml, src layout, and test structure
