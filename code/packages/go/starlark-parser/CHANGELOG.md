# Changelog

All notable changes to the starlark-parser package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-19

### Added
- `CreateStarlarkParser()` function that tokenizes source, loads the `starlark.grammar`, and returns a configured `GrammarParser`
- `ParseStarlark()` convenience function for one-shot parsing of Starlark source strings into an AST
- Comprehensive test suite with 10 test functions covering: simple assignment, arithmetic expressions, function definitions, if/else conditionals, for loops, BUILD file patterns, multiple statements, list literals, dict literals, and the factory function
