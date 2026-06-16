# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-16

### Added

- Initial release of the R lexer crate — the first piece of the R frontend,
  built as a sibling of the historical-S `s-lexer`.
- `tokenize_r()` / `try_tokenize_r()` and the `create_r_lexer()` factory.
- Embedded `r.tokens` grammar (`src/_grammar.rs`), generated ahead of time.
- R's lexical departures from S: `_` is a name character (no `UNDERSCORE`
  token; `NAME` includes `_`), the right super-assignment `->>`, and the typed
  `NA_integer_` / `NA_real_` / `NA_character_` constants.
- Shared-with-S surface: `<- <<- ->`, comparisons, arithmetic, `:`, `%op%`,
  `$`, brackets, keywords, both string-quote styles, `#` comments, and the
  `drop_bracketed_newlines` hook (newlines insignificant inside `( )`/`[ ]`,
  significant inside `{ }` and at top level).
