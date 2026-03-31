# Changelog — CodingAdventures::RubyParser

## [0.01] — 2026-03-29

### Added
- Initial implementation of the hand-written recursive-descent Ruby parser.
- `new($source)` constructor: tokenizes with `CodingAdventures::RubyLexer`.
- `parse()` method: returns the root `ASTNode` (rule_name `"program"`). Dies on error.
- `parse_ruby($source)` class method: tokenize and parse in one call.
- `CodingAdventures::RubyParser::ASTNode` — lightweight AST node class with
  inner nodes (rule_name + children arrayref) and leaf nodes (token wrapper).
- Supported constructs:
  - Assignments: `x = 5`
  - Method definitions: `def greet(name) ... end`
  - Class definitions: `class Dog ... end`
  - If/elsif/else statements with `end`-delimited block handling
  - Unless statements: `unless condition ... end`
  - While loops: `while expr ... end`
  - Until loops: `until expr ... end`
  - Return statements: `return value`
  - Method calls with parens: `puts("hello")`
  - Method calls without parens (keyword-style): `puts "hello"`
  - Expressions with full operator precedence:
    equality (==/!=) > comparison (</>/<=/>= ) > additive (+/-) > multiplicative (*/ ) > unary (-) > primary
- `BUILD` and `BUILD_windows` for the monorepo build system.
- `Makefile.PL` and `cpanfile` for CPAN distribution.
- Test suite in `t/00-load.t` (module load) and `t/01-basic.t` (comprehensive).
- `README.md` with usage examples, AST node type reference, and Ruby vs Python comparison.
