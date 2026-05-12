# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-05-11

### Added

Source-level negation-as-failure now parses end-to-end:

- New `naf_goal = NAF goal_primary` production in the ISO Prolog
  parser grammar. `goal_primary` gains a `naf_goal` alternative ahead
  of `equality_goal` / `callable_goal`.
- `ast_to_term` lowers `naf_goal` into the canonical
  `compound("\\+", vec![inner])` shape that
  `prolog_loader::naf_or_pos` already pattern-matches on, so the
  loader→engine pipeline turns it into `BodyLiteral::Neg` without
  further changes.
- Same `naf_goal` production added to the SWI-Prolog dialect grammar
  (`code/grammars/prolog/swi.grammar`).

`code/grammars/prolog/iso.grammar` is canonical; the embed is
regenerated via
`cargo run -p prolog-parser --example regenerate_grammar`.

## [0.1.0] - 2026-05-11

### Added

- `create_iso_prolog_parser(source)` — builds a `parser::GrammarParser`
  wired to the ISO Prolog parser grammar (compiled from
  `code/grammars/prolog/iso.grammar`) and the prolog-lexer's tokens.
- `parse_iso_prolog(source)` — convenience wrapper that returns the
  top-level `program` AST node (`GrammarASTNode`). Panics on parse
  errors, matching the convention of other `*-parser` crates.
- `try_parse_iso_prolog(source)` — returns a `Result<GrammarASTNode,
  GrammarParseError>` for recoverable error handling.
- `ast_to_term(node, var_map)` — lowers a term-rooted
  `GrammarASTNode` into a `logic_core::Term`. Handles atoms, numbers,
  strings, variables (with identity shared within a clause via
  `var_map`), anonymous variables (fresh per occurrence), compound
  terms, and lists (encoded as canonical `'.'/2` + `[]` cons cells).
- `collect_clauses_and_queries(program)` — walks the top-level
  `program → statement*` tree and emits a `Vec<ProgramItem>` of
  `Fact`, `Rule { head, body }`, or `Query(body)` items. Each item
  gets its own variable map so variable identity is clause-local
  (Prolog semantics).
- `src/_grammar.rs` — auto-generated `ParserGrammar` embedding of
  `iso.grammar`. Regenerate with
  `cargo run -p prolog-parser --example regenerate_grammar`.
- 11 tests covering: bare-atom and compound facts, rules with single
  and multi-literal bodies, queries with conjunction, variable
  identity (`p(X, X)` shares ids), list lowering (with and without
  tail), integer/float terms, a full small program, and a parse
  error case.

### Architecture

Mirrors the Python `iso-prolog-parser`: same `iso.grammar` file, same
`grammar-tools::ParserGrammar` shape, same `GrammarParser` machinery.
The Python ecosystem keeps operator-precedence parsing in a separate
`prolog-operator-parser` crate; the Rust equivalent is a planned
follow-up. This first slice accepts the canonical functional form
for non-trivial expressions (e.g. `'+'(X, '*'(Y, Z))` rather than
`X + Y * Z`).

### Not in this slice

- Operator-precedence resolution (`X + Y * Z`, `X = 1 + 2`).
- User-defined operator directives.
- DCG transformations (`-->`) lower to a placeholder Fact for now.
- Negation-as-failure `\+ G` is parsed as the compound `'\+'(G)`;
  the downstream `prolog-loader` will translate this into the
  engine's `BodyLiteral::Neg`.
