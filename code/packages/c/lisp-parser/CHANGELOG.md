# Changelog

All notable changes to the C `lisp-parser` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- AST child accessors `lp_sexpr_child_count` / `lp_sexpr_child` /
  `lp_sexpr_dotted_last` / `lp_sexpr_quoted_inner`, so tree walkers (e.g. the
  `lisp-compiler`) can traverse an opaque `LpSExpr`.

## [0.1.0] - 2026-07-13

### Added

- Initial pure-ISO C17 port of the Rust `lisp-parser` crate — a
  recursive-descent parser turning the lexer's token stream into an
  S-expression AST. Depends on the sibling `c/lisp-lexer` (compiled in via
  `run.sh`).
- `lp_parse(source, out, err)` and `lp_parse_tokens(tokens, n, out, err)`
  producing an owned `LpProgram`; `lp_program_free` / `lp_strlist_free`.
- `LpSExpr` node kinds Atom (number/symbol/string) / List / DottedPair / Quoted,
  with inspection helpers `lp_sexpr_kind`, `lp_sexpr_atom_kind` /
  `lp_sexpr_atom_value`, and `find_atoms` / `count_lists` / `count_quoted` at
  both the node and program level.
- Owned tagged-union AST with careful teardown on parse failure; EOF-safe
  peeking so a truncated stream never reads out of bounds; overflow-guarded
  growth.
- 40 checks mirroring the Rust crate's own unit tests, run under every available
  C compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
