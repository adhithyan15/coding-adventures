# Changelog — CodingAdventures::AlgolLexer (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-04-06

### Added

- Initial implementation of `CodingAdventures::AlgolLexer`.
- `tokenize($source)` — tokenizes an ALGOL 60 string using rules compiled from
  the shared `algol.tokens` grammar file.
- Grammar is read from `code/grammars/algol.tokens` once and cached in
  package-level variables (`$_grammar`, `$_rules`, `$_skip_rules`, `$_keywords`).
- Path navigation uses `File::Basename::dirname` and `File::Spec::rel2abs`
  relative to `__FILE__`, climbing 5 directory levels to the repo root
  (same depth as json-lexer).
- Skip patterns consume whitespace and ALGOL 60 comments (`comment ... ;`)
  silently; no WHITESPACE or COMMENT tokens are emitted.
- Keyword reclassification: any IDENT whose lowercase value appears in the
  keyword table is promoted to the corresponding keyword type (e.g., `begin`
  → `BEGIN`, `BEGIN` → `BEGIN`, `Begin` → `BEGIN`).
- Partial keyword matches produce IDENT: `beginning` → `IDENT`, not `BEGIN`.
- Multi-character operator priority: `:=` before `:`, `**` before `*`,
  `<=` before `<`, `>=` before `>`, `!=` before any `!` use.
- `REAL_LIT` matched before `INTEGER_LIT` so `3.14` is not split.
- All 27 ALGOL 60 keywords supported: `begin`, `end`, `if`, `then`, `else`,
  `for`, `do`, `step`, `until`, `while`, `goto`, `switch`, `procedure`,
  `own`, `array`, `label`, `value`, `integer`, `real`, `boolean`, `string`,
  `true`, `false`, `not`, `and`, `or`, `impl`, `eqv`, `div`, `mod`, `comment`.
- Line and column tracking for all tokens.
- `die` with a descriptive "LexerError" message on unexpected input.
- `t/00-load.t` — smoke test that the module loads and has a VERSION.
- `t/01-basic.t` — comprehensive test suite covering all keywords,
  case-insensitivity, keyword boundary disambiguation, all literal types,
  all operators (multi-char and single-char), all delimiters, comment
  skipping, whitespace handling, composite programs, position tracking,
  and error handling.
- `BUILD` and `BUILD_windows` scripts.
- `Makefile.PL` and `cpanfile`.
