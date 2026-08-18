# Changelog — CodingAdventures::JsonLexer (Perl)

All notable changes to this package are documented here.

## [0.02] — 2026-08-17

### Changed

- Eliminated runtime grammar-file disk reads. `_grammar()` now `require`s a
  checked-in generated module, `CodingAdventures::JsonLexer::_Grammar`
  (compiled ahead of time from `code/grammars/json/json.tokens` via
  `grammar-tools.pl compile-tokens`), instead of opening and parsing
  `json.tokens` off disk on first use. The old path climbed out of the
  installed package's own directory to a monorepo-relative
  `code/grammars/json/json.tokens` path that a real CPAN install of this
  package does not ship, so first use after `cpanm install` would have died
  with "cannot open ... No such file or directory". `_grammars_dir()` is
  removed along with the now-unused `File::Basename`/`File::Spec` imports.
  Public API (`tokenize`), token types, and error messages are unchanged.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::JsonLexer`.
- `tokenize($source)` — tokenizes a JSON string using rules compiled from
  the shared `json.tokens` grammar file.
- Grammar is read from `code/grammars/json.tokens` once and cached in
  package-level variables (`$_grammar`, `$_rules`, `$_skip_rules`).
- Path navigation uses `File::Basename::dirname` and `File::Spec::rel2abs`
  relative to `__FILE__`, climbing 5 directory levels to the repo root.
- Skip patterns (whitespace) are consumed silently; no WHITESPACE tokens
  are emitted.
- Token types mirror the `json.tokens` grammar: STRING, NUMBER, TRUE,
  FALSE, NULL, LBRACE, RBRACE, LBRACKET, RBRACKET, COLON, COMMA, EOF.
- Alias resolution: definitions with `-> ALIAS` syntax emit the alias name.
- Line and column tracking for all tokens.
- `die` with a descriptive "LexerError" message on unexpected input.
- `t/00-load.t` — smoke test that the module loads and has a VERSION.
- `t/01-basic.t` — comprehensive test suite covering all token types,
  escape sequences, whitespace, position tracking, nested structures,
  real-world JSON, and error handling.
- `BUILD` and `BUILD_windows` scripts.
