# Changelog — CodingAdventures::ExcelLexer

## 0.01 — 2026-03-29

### Added
- Initial implementation of `CodingAdventures::ExcelLexer`.
- Grammar-driven tokenizer reading `excel.tokens` from the shared grammars directory.
- Case normalization: source is lowercased before tokenizing (handles Excel's
  `@case_insensitive true`); returned token values are lowercase.
- Emits all Excel formula token types: `EQUALS`, `CELL`, `NAME`, `NUMBER`,
  `STRING`, `TRUE`, `FALSE`, `ERROR_CONSTANT`, `REF_PREFIX`,
  `STRUCTURED_KEYWORD`, `STRUCTURED_COLUMN`, `SPACE`, operator tokens, `EOF`.
- SPACE tokens preserved (not skipped) because space is the range-intersection
  operator in Excel; only tabs, CR, LF are silently consumed.
- Grammar object cached after first load for efficiency.
- `t/00-load.t` — module load test.
- `t/01-basic.t` — comprehensive tokenization tests covering all token types
  and composite formulas.
- `Makefile.PL`, `cpanfile`, `required_capabilities.json`,
  `CHANGELOG.md`, `README.md`, `BUILD`.
