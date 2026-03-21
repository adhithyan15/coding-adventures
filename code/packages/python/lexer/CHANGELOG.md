# Changelog

All notable changes to the lexer package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
