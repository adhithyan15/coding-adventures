# Changelog

All notable changes to this project will be documented in this file.

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
