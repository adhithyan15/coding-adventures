# Changelog — coding-adventures-excel-lexer

## 0.1.0 — 2026-03-29

### Added
- Initial implementation of `coding_adventures.excel_lexer`.
- Loads `excel.tokens` grammar from the shared `code/grammars/` directory.
- Wraps the grammar-driven `GrammarLexer` from `coding-adventures-lexer`.
- Normalizes input to lowercase before tokenizing (handles Excel's
  `@case_insensitive true` declaration via pre-processing).
- Emits all Excel formula token types: `EQUALS`, `CELL`, `NAME`, `NUMBER`,
  `STRING`, `TRUE`, `FALSE`, `ERROR_CONSTANT`, `REF_PREFIX`,
  `STRUCTURED_KEYWORD`, `STRUCTURED_COLUMN`, `SPACE`, operator tokens, `EOF`.
- Exposes `M.tokenize(source)` and `M.get_grammar()`.
- Grammar object cached after first load (no repeated file I/O).
- Comprehensive busted test suite covering all token types and composite formulas.
- `required_capabilities.json` declaring `filesystem:read`.
- `CHANGELOG.md`, `README.md`, and `BUILD` file.
