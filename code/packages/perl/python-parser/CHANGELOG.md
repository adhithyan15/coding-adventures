# Changelog — CodingAdventures::PythonParser

## [0.01] — 2026-03-29

### Added
- Initial implementation of the hand-written recursive-descent Python parser.
- `new($source)` constructor: tokenizes with `CodingAdventures::PythonLexer`.
- `parse()` method: returns the root `ASTNode` (rule_name `"program"`). Dies on error.
- `parse_python($source)` class method: tokenize and parse in one call.
- `CodingAdventures::PythonParser::ASTNode` — lightweight AST node class with
  inner nodes (rule_name + children arrayref) and leaf nodes (token wrapper).
- Supported constructs:
  - Assignments: `x = 5`
  - Function definitions: `def add(a, b):\n    return a + b`
  - If/elif/else statements with INDENT/DEDENT block handling
  - For loops: `for i in range(10):`
  - While loops: `while x == 0:`
  - Return statements: `return value`
  - Import statements: `import math`
  - From-import statements: `from math import sqrt`
  - Function calls: `print("hello")`
  - Expressions with full operator precedence:
    comparison (==) > additive (+/-) > multiplicative (*//) > unary (-) > primary
- `BUILD` and `BUILD_windows` for the monorepo build system.
- `Makefile.PL` and `cpanfile` for CPAN distribution.
- Test suite in `t/00-load.t` (module load) and `t/01-basic.t` (comprehensive).
- `README.md` with usage examples and AST node type reference.
