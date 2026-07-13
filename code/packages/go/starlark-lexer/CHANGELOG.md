# Changelog

All notable changes to the starlark-lexer package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-07-13

### Changed

- The token grammar is now embedded at compile time as native Go data in
  `grammar_data.go` and consumed directly by the loader; the package no longer
  reads `code/grammars/**` at run time via `runtime.Caller`. This drops the
  filesystem capability (empty `required_capabilities.json`, `gen_capabilities.go`
  removed), lets the package build and run standalone, and fixes the previously
  dead embedded grammar (the old `_grammar.go` name is ignored by the Go
  toolchain because of its leading underscore).

## [0.1.0] - 2026-03-19

### Added
- `CreateStarlarkLexer()` function that loads the `starlark.tokens` grammar and returns a configured `GrammarLexer` in indentation mode
- `TokenizeStarlark()` convenience function for one-shot tokenization of Starlark source strings
- Comprehensive test suite with 12 test functions covering: simple expressions, keywords, reserved keywords, indentation (INDENT/DEDENT), bracket suppression, multi-character operators, string literals, comments, float literals, hex/octal integers, augmented assignment, and the factory function
