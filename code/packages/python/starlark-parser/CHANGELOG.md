# Changelog

All notable changes to the Starlark Parser package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the Starlark parser package.
- `parse_starlark()` function that parses Starlark source code into a generic AST.
- `create_starlark_parser()` factory function for creating a `GrammarParser` configured for Starlark.
- Starlark parser grammar file (`starlark.grammar`) with support for:
  - File structure (sequences of statements separated by newlines)
  - Simple statements: assignment, augmented assignment, return, break, continue, pass, load
  - Compound statements: if/elif/else, for loops, function definitions with indented suites
  - Full expression precedence: lambda, ternary if/else, or, and, not, comparisons, bitwise ops, shifts, arithmetic, unary, exponentiation, primary
  - Primary expressions with suffixes: attribute access, indexing, slicing, function calls
  - Atoms: integers, floats, strings, names, True/False/None, lists, dicts, parenthesized expressions
  - Comprehensions: list and dict comprehensions with for/if clauses
  - Function parameters: positional, default values, *args, **kwargs
  - Function call arguments: positional, keyword, *args unpacking, **kwargs unpacking
- Comprehensive test suite with 80%+ coverage.
