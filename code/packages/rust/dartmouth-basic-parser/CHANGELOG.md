# Changelog — coding-adventures-dartmouth-basic-parser

All notable changes to this crate will be documented in this file.

## [0.4.0] — 2026-07-14

### Fixed — recursion-depth guard against native stack overflow (DoS)

`create_dartmouth_basic_parser` built its `GrammarParser` with no
recursion-depth cap, even though this crate is reachable via the
`lang-aot` multi-language driver on arbitrary source files — a real, not
theoretical, attack surface. Deeply-nested input, in any of this
grammar's three *independent* recursive shapes (paren/function-call
nesting, direct `power` (`^`) self-recursion, array-index nesting), would
recurse until it overflowed the native thread stack — an uncatchable
process abort — before this crate's own `Result`-returning entry points
ever got a chance to report anything.

All three shapes were independently measured (binary search, uncapped
parser, the true default per-test-thread stack — no `RUST_MIN_STACK`
override, no explicit `Builder::stack_size`, matching what `cargo test`
and a production caller both actually get — debug build, adversarial
5000-level input): paren/function-call and array-index nesting safe
through 270 rule-frames, crashes at 280; `power` self-recursion (the
*binding*, lower floor) safe through 175, crashes at 176. Added a bespoke
`MAX_RULE_DEPTH = 120` — about 31% below the binding floor — and wired it
into `create_dartmouth_basic_parser` via `.with_max_depth(...)`.

- Added `MAX_RULE_DEPTH: usize = 120` and wired it into
  `create_dartmouth_basic_parser`.
- 9 new regression tests (3 per independent recursive shape): deep
  adversarial input on an enlarged-stack thread returns a clean `Err`,
  input at the measured real-nesting boundary (22 levels for
  paren/function-call, 111 for `power`-chain, 18 for array-index) still
  parses while one level past it doesn't, and the cap trips before the
  native stack would overflow even on a default-stack thread.

No change to behaviour for any input that nests below the cap.

## [0.3.0] — 2026-07-02 — BA-DIM-2D: multi-dimensional array subscripts

### Changed

- **Grammar** (`code/grammars/dartmouth_basic.grammar`): two rules gain a
  comma-separated repetition so multi-dimensional arrays parse:
  - `dim_decl = NAME LPAREN NUMBER { COMMA NUMBER } RPAREN` — `DIM A(m,n)`
  - `variable = NAME LPAREN expr { COMMA expr } RPAREN | NAME` — `A(i,j)`
- Regenerated `src/_grammar.rs` from the updated grammar via
  `grammar-tools generate-rust-compiled-grammars dartmouth_basic`.  A scalar
  `NAME` and a 1-D `A(i)` still parse exactly as before (the repetition matches
  zero extra subscripts).

## [0.2.0] — 2026-06-27

### Added

- `STRING` is now a primary expression so `LET A$ = "HI"` and string equality
  forms have a normal expression AST.
- Parser tests now cover `$`-suffixed string variables in `LET` and `PRINT`.

## [0.1.0] — 2026-04-10

### Added

- Initial implementation of the Dartmouth BASIC parser.
- `parse_dartmouth_basic(source: &str) -> GrammarASTNode` — one-call entry
  point that tokenizes the source and parses it into an AST with root rule
  `"program"`.
- `create_dartmouth_basic_parser(source: &str) -> GrammarParser` — factory
  function that returns a configured `GrammarParser` for callers that need
  fine-grained control over the parse step.
- Grammar path resolution via `env!("CARGO_MANIFEST_DIR")` pointing to
  `code/grammars/dartmouth_basic.grammar`.
- Complete test suite covering all 17 statement types:
  - LET (scalar and array element assignment)
  - PRINT (bare, expression, string, comma separator, semicolon separator)
  - INPUT (single and multiple variables)
  - IF-THEN (all 6 relational operators: =, <, >, <=, >=, <>)
  - GOTO
  - GOSUB / RETURN
  - FOR / NEXT (with and without STEP)
  - END / STOP
  - REM
  - READ / DATA / RESTORE
  - DIM (single and multiple declarations)
  - DEF (user-defined function)
- Expression tests: addition, subtraction, multiplication, division,
  exponentiation (right-associative), unary minus, parentheses.
- All 11 built-in function tests: SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR,
  INT, RND, SGN.
- User-defined function tests: FNA, FNZ.
- Array subscript in expressions: A(I), A(I+1).
- Multi-line program tests: hello world, counting loop, conditional,
  subroutine.
- Edge case test: bare line number `"10\n"` is valid BASIC.
- Factory function test: `create_dartmouth_basic_parser` returns a working
  parser.
- READ/DATA round-trip program test.
- Complex expression precedence test.
- Literate programming style with detailed inline comments explaining
  1964 Dartmouth BASIC history and the grammar-driven parser approach.
