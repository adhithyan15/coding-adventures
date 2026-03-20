# Changelog

All notable changes to the starlark-lexer package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-19

### Added
- `CreateStarlarkLexer()` function that loads the `starlark.tokens` grammar and returns a configured `GrammarLexer` in indentation mode
- `TokenizeStarlark()` convenience function for one-shot tokenization of Starlark source strings
- Comprehensive test suite with 12 test functions covering: simple expressions, keywords, reserved keywords, indentation (INDENT/DEDENT), bracket suppression, multi-character operators, string literals, comments, float literals, hex/octal integers, augmented assignment, and the factory function
