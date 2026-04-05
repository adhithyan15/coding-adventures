# Changelog — CodingAdventures::TypescriptParser

All notable changes to this package are documented here.

## [0.02] — 2026-04-05

### Added

- `new($source, $version)` — optional `$version` argument passed through to
  `CodingAdventures::TypescriptLexer->tokenize($source, $version)`.
- `parse_ts($source, $version)` — convenience class method updated to accept
  optional version.
- Valid version strings: `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`,
  `"ts5.0"`, `"ts5.8"`. Passing `undef` or `""` uses the generic grammar
  (backward compatible).
- Extended test suite: new version-aware subtests in `t/01-basic.t` covering
  all 6 recognized versions, `new($source, $version)`, and error cases.

### Changed

- `$VERSION` bumped from `0.01` to `0.02`.

## [0.01] — 2026-03-29

### Added

- Initial implementation of the TypeScript parser (hand-written recursive descent).
- Tokenizes with `CodingAdventures::TypescriptLexer` and builds an AST.
- Supported constructs: variable declarations (var/let/const), assignments,
  function declarations, if/else, for loops, return statements, blocks,
  function calls, arrow functions, and full expression grammar.
- Operator precedence: equality → comparison → additive → multiplicative → unary → primary.
- `CodingAdventures::TypescriptParser::ASTNode` — leaf and inner node class.
- `parse_ts($source)` convenience class method.
- `t/00-load.t` — module load and version tests.
- `t/01-basic.t` — comprehensive grammar coverage tests.
- `Makefile.PL`, `cpanfile`, `BUILD`, `README.md`, `CHANGELOG.md`,
  and `required_capabilities.json`.
