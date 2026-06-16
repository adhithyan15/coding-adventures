# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-16

### Added

- Initial release of the R parser crate — item R-2 of the R frontend.
- `parse_r()` / `try_parse_r()` and the `create_r_parser()` factory, producing a
  `GrammarASTNode` rooted at the `program` rule.
- Embedded `r.grammar` (`src/_grammar.rs`), generated ahead of time.
- `r.grammar` mirrors `s.grammar`'s rule names exactly so the shared `s-runtime`
  tree-walker can evaluate R programs unchanged. The grammar differences from S:
  - `=` and `->>` are assignment operators (alongside `<-`, `<<-`, `->`);
  - the typed-`NA` atoms `NA_integer_` / `NA_real_` / `NA_character_`.
- 11 tests covering R's assignment operators, the `=` named-arg vs assignment
  distinction, typed NAs, the shared precedence cascade, indexing/`[[`/`$`,
  functions, control flow, multi-line input, and error reporting.
