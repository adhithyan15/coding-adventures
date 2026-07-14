# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-07-14

### Fixed — recursion-depth guard against native stack overflow (DoS)

`create_algol_parser` built its `GrammarParser` with no recursion-depth
cap, even though this crate is reachable via the `lang-aot` multi-language
driver on arbitrary source files — a real, not theoretical, attack
surface. Deeply-nested input, in any of this grammar's eleven
*independent* recursive shapes (the richest grammar in this depth-cap
sweep — arithmetic conditional-expression chains, NOT-chains, begin/end
block nesting, if/else statement nesting, designational-expression
parens, array-subscript nesting, arithmetic/boolean/unified-expression
parens, nested procedure declarations, and for-loop body nesting), would
recurse until it overflowed the native thread stack — an uncatchable
process abort — before this crate's own `Result`-returning entry points
ever got a chance to report anything.

All eleven shapes were independently measured (binary search, uncapped
parser, the true default per-test-thread stack — no `RUST_MIN_STACK`
override, no explicit `Builder::stack_size`, matching what `cargo test`
and a production caller both actually get — debug build, adversarial
5000-level input). If/else statement nesting (`statement -> cond_stmt ->
statement`) is the *binding* (lowest) floor, safe through 197 rule-frames,
crashes at 198. Added a bespoke `MAX_RULE_DEPTH = 135` — about 31% below
the binding floor — and wired it into `create_algol_parser` via
`.with_max_depth(...)`. See the `MAX_RULE_DEPTH` doc comment in
`src/lib.rs` for the full per-shape floor table.

- Added `MAX_RULE_DEPTH: usize = 135` and wired it into
  `create_algol_parser`.
- 33 new regression tests (3 per independent recursive shape): deep
  adversarial input on an enlarged-stack thread returns a clean `Err`,
  input at the measured real-nesting boundary still parses while one
  level past it doesn't, and the cap trips before the native stack would
  overflow even on a default-stack thread.
- Incidentally discovered (not fixed here — a pre-existing, unrelated
  bug, flagged separately): the checked-in `src/_grammar.rs` is stale
  relative to `code/grammars/algol/algol60.grammar`. The `.grammar`
  source documents an if/then/else conditional-expression form on the
  unified `expression` rule, and broader (both-branches) recursion on
  `arith_expr`/`bool_expr`/`desig_expr`'s conditional forms, neither of
  which is present in the currently-compiled parser. This depth-cap fix
  was calibrated against what the parser *actually* runs today, not the
  aspirational grammar-file text.

No change to behaviour for any input that nests below the cap.

## [0.2.0] - 2026-06-22

### Added

- **`own` declaration prefix (LANG-FULL AL6).** `type_decl` now accepts an
  optional leading `own` keyword (`[ "own" ] type ident_list`), e.g.
  `own integer n` — an ALGOL 60 §5.2.5 static-lifetime variable. The keyword
  was already in `algol.tokens` / the lexer's keyword set; this wires it into
  the parser grammar so the frontend can give it static-lifetime semantics.

### Notes

- The optional was added to **both** `code/grammars/algol.grammar` and the
  compiled `src/_grammar.rs` (surgically, for the `type_decl` rule only). A
  full `grammar-tools compile-grammar` regen was deliberately **not** run: the
  checked-in `algol.grammar` has drifted ahead of the compiled grammar in
  unrelated rules (`for_stmt` loop targets, optional `actual_params`, labels)
  that the IIR frontend does not yet support, so regenerating wholesale would
  pull those in and break parsing. Resyncing the full grammar is tracked as
  separate follow-up work.

## [0.1.0] - 2026-04-06

### Added

- Initial release of the ALGOL 60 parser crate.
- `create_algol_parser()` factory function returning a `GrammarParser` configured for ALGOL 60.
- `parse_algol()` convenience function returning `GrammarASTNode` directly.
- Loads the `algol.grammar` file at runtime from the shared `grammars/` directory.
- Full ALGOL 60 grammar support: program, block, declarations (type, array, switch, procedure), statements (assign, conditional, for, goto, proc call, compound, empty), expressions (arithmetic with operator precedence, boolean with eqv/impl/or/and/not, designational).
- Depends on `coding-adventures-algol-lexer` for tokenization.
- 16 unit tests covering: minimal program, block structure, assignment, arithmetic expression, if/then, if/then/else, for loop (step/until form), type declaration, real declaration, factory function, compound statement, exponentiation (`**` and `^`), boolean expressions, goto, procedure call, and for loop (while form).
