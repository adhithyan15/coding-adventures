# Changelog

All notable changes to `coding_adventures_python_parser` will be documented in this file.

## [0.2.0] - 2026-04-12

### Added
- `parse(source, version: "3.12")` now accepts a Python version selector
- `DEFAULT_VERSION` constant (`"3.12"`)
- `SUPPORTED_VERSIONS` constant listing supported parser versions
- `grammar_path(version)` helper for versioned parser grammars
- Versioned compiled parser artifacts for `"2.7"`, `"3.0"`, `"3.6"`, `"3.8"`, `"3.10"`, and `"3.12"`

### Changed
- Removed the old unversioned `python.grammar` parser path from the Ruby package
- The Ruby parser now always tokenizes and parses with matching versioned Python grammars

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
