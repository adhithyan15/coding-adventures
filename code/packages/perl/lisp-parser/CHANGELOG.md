# Changelog — CodingAdventures::LispParser

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01 — 2026-03-29

Initial release.

- Hand-written recursive-descent parser implementing all six Lisp grammar rules:
  program, sexpr, atom, list, list_body, quoted.
- Handles atoms (NUMBER, SYMBOL, STRING), empty lists, proper lists, nested
  lists of arbitrary depth, quoted forms (tick shorthand), dotted pairs
  (cons cell notation), and multi-expression programs.
- `CodingAdventures::LispParser::ASTNode` submodule for the AST node type.
- Full Test2::V0 test suite covering all grammar productions, multi-expression
  programs, ASTNode accessors, and error cases.
