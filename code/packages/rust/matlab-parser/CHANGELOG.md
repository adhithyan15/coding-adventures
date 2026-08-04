# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-07-11

### Fixed

- **DoS hardening**: `create_matlab_parser` / `try_parse_matlab` now opt the
  underlying `GrammarParser` into a recursion-depth cap
  (`MAX_RULE_DEPTH = 200`) via `.with_max_depth(...)`. Previously the parser
  recursed once per nested `(...)` layer with no limit; deeply nested input
  (thousands of levels) could overflow the *native* thread stack — an
  uncatchable process abort — before ever reaching a `Result`-returning entry
  point. Now such input cleanly returns a `String` error instead of crashing
  the host process.
- The `200` cap was derived empirically (not copied from
  `parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`, which is tuned for a much
  shallower ECMAScript-shaped grammar and would have allowed only ~7 real
  nesting levels here): a throwaway, isolated subprocess binary-searched, on
  a default ~2 MiB stack worker thread, for the largest `with_max_depth`
  value that still returns a clean error instead of overflowing. See the
  `MAX_RULE_DEPTH` doc comment in `src/lib.rs` for the full derivation
  (rule-chain analysis + measured crash floor).
- Added 3 regression tests exercising the guard on the real MATLAB grammar: a
  big-stack deep-nesting test, a default-stack deep-nesting test (proving the
  cap trips before the native stack would overflow), and a boundary test
  proving legitimate nesting up to 12 levels still parses while 13 trips the
  cap.

## [0.1.0] - 2026-06-16

### Added

- Initial release of the MATLAB parser crate — item **MA-3c** of the MATLAB
  frontend on `array-runtime` (spec
  [`MA01`](../../../specs/MA01-matlab-language.md)), built as a sibling of the
  S/R parsers (a thin wrapper over the generic `GrammarParser`).
- `parse_matlab()` / `try_parse_matlab()` and the `create_matlab_parser()`
  factory; produces a `GrammarASTNode` tree keyed by grammar rule name.
- Embedded `matlab.grammar` (`src/_grammar.rs`), generated ahead of time, with
  the full MATLAB precedence cascade (`=`, `||`, `&&`, `|`, `&`, comparison,
  colon, additive, multiplicative incl. element-wise, unary, power, postfix),
  matrix/cell literals (`[1 2; 3 4]`, `{…}`, juxtaposed or comma columns, `;`/
  newline rows), postfix transpose `'`/`.'`, calls/indexing `A(i,j)`,
  whole-dimension `A(:,k)`, cell indexing `C{i}`, field access `s.field`,
  control flow (`if`/`for`/`while`/`switch`/`try`/`break`/`continue`/`return`/
  `global`), `function … end` (incl. `[a,b] =` returns), and anonymous `@(x) …`.
- **The `end` disambiguation**: a pre-parse hook (`retag_index_end`) rewrites
  every `end` inside `( )`/`[ ]`/`{ }` to a `NAME` before parsing, so the `"end"`
  block terminators and the `A(end)` index sentinel never collide.
- 16 unit tests + 1 doctest covering the precedence cascade, matrix/cell
  literals (rows via `;` and newline), transpose, calls/indexing/fields, `end`
  as both sentinel and terminator, every control-flow construct, anonymous
  functions, and statement terminators. 100% line coverage of the crate logic.
