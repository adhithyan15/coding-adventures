# Changelog

All notable changes to `coding_adventures_python_parser` will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- Initial release
- `CodingAdventures::PythonParser.parse(source)` method that parses Python source code into ASTs
- Loads `python.grammar` and delegates to `GrammarDrivenParser`
- Supports assignments: `x = 1 + 2`
- Supports operator precedence via grammar rule hierarchy
- Supports parenthesized expressions: `(1 + 2) * 3`
- Supports multiple statements separated by newlines
- Supports string literals, numeric literals, and variable references
- Full test suite with SimpleCov coverage >= 80%
