# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Initial implementation of ECMAScript 3 (1999) lexer
- Grammar-driven tokenization using es3.tokens
- Support for strict equality (===, !==), regex literals, try/catch/finally/throw, instanceof
- Convenience functions: `tokenize/1`, `load_grammar/0`, `grammar_path/0`
