# Changelog

All notable changes to `coding_adventures_algol_parser` will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `parse` now loads the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`) instead of reading and parsing `algol60.grammar` from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.
- Regenerated `_grammar.rb` from the current `algol60.grammar` source — the previously-committed compiled file was stale (never wired up, so drift went unnoticed) and caused parse failures once actually loaded.

## [0.1.0] - 2026-04-06

### Added
- Initial release
- `CodingAdventures::AlgolParser.parse(source)` method that parses ALGOL 60 source text into an AST
- Loads `algol.grammar` grammar file and delegates to `GrammarDrivenParser`
- Returns `CodingAdventures::Parser::ASTNode` tree rooted at `"program"`
- Supports full ALGOL 60 block structure: `BEGIN { declaration ; } statement { ; statement } END`
- Supports all declaration forms: `type_decl`, `array_decl`, `switch_decl`, `procedure_decl`
- Supports all statement forms: `assign_stmt`, `cond_stmt`, `for_stmt`, `goto_stmt`, `proc_stmt`, `compound_stmt`, `empty_stmt`
- Supports arithmetic expressions with full operator precedence (exponentiation > mult > add)
- Supports boolean expressions with full operator precedence (eqv < impl < or < and < not)
- Dangling-else ambiguity resolved at the grammar level (then-branch is `unlabeled_stmt`)
- Left-associative exponentiation per the ALGOL 60 report: `2**3**4 = (2**3)**4`
- Conditional expressions in both arithmetic and boolean expression positions
- Designational expressions for computed goto
- Full test suite with SimpleCov coverage >= 80%
