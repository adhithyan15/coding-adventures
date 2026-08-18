# Changelog — CodingAdventures::JavascriptLexer (Perl)

All notable changes to this package are documented here.

## [0.03] — 2026-08-17

### Changed
- Grammar is now loaded from precompiled `CodingAdventures::JavascriptLexer::_Grammar*`
  modules (one per ECMAScript version, plus a generic default) instead of
  reading and parsing `.tokens` files off disk at runtime. A published CPAN
  distribution would not include the monorepo's `code/grammars/` tree, so
  the old path-climbing `open()` would fail after install. The 15 compiled
  modules (`javascript.tokens` + es1, es3, es5, es2015-es2025) are generated
  via `grammar-tools compile-tokens` and checked into git. Dropped the
  now-unused `File::Basename`/`File::Spec` dependencies.

## [0.02] — 2026-04-05

### Added

- `tokenize($source, $version)` — optional `$version` parameter selects
  a versioned grammar file under `code/grammars/ecmascript/`.
- Valid version strings: `"es1"`, `"es3"`, `"es5"`, `"es2015"`.."es2025"`.
  Passing `undef` or `""` uses the generic `javascript.tokens` grammar
  (backward compatible).
- `_resolve_tokens_path($version)` — internal helper that maps version
  strings to grammar file paths.
- Per-version caches for grammar, compiled rules, skip rules, and keyword
  map (hashes keyed by version string instead of single package variables).
- Validation: unknown version strings raise a descriptive `die` immediately.
- Extended test suite: new version-aware subtests in `t/01-basic.t` covering
  ES1/ES3/ES5/ES2015/ES2020/ES2025 versions, cache consistency, and error
  cases.

### Changed

- `$VERSION` bumped from `0.01` to `0.02`.
- Package-level cache variables (`$_grammar`, `$_rules`, etc.) replaced by
  hash-based per-version caches (`%_grammar_cache`, etc.).
- `_grammar()` and `_build_rules()` now accept a `$version` argument.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::JavascriptLexer`.
- `tokenize($source)` — tokenizes a JavaScript string using rules compiled
  from the shared `javascript.tokens` grammar file.
- Grammar is read from `code/grammars/javascript.tokens` once and cached in
  package-level variables (`$_grammar`, `$_rules`, `$_skip_rules`).
- Path navigation uses `File::Basename::dirname` and `File::Spec::rel2abs`
  relative to `__FILE__`, climbing 5 directory levels to the repo root.
- Skip patterns (whitespace) are consumed silently; no WHITESPACE tokens
  are emitted.
- Token types: NAME, NUMBER, STRING; keyword types: LET, CONST, VAR, IF,
  ELSE, WHILE, FOR, DO, FUNCTION, RETURN, CLASS, IMPORT, EXPORT, FROM, AS,
  NEW, THIS, TYPEOF, INSTANCEOF, TRUE, FALSE, NULL, UNDEFINED; operator
  types: STRICT_EQUALS, STRICT_NOT_EQUALS, EQUALS_EQUALS, NOT_EQUALS,
  LESS_EQUALS, GREATER_EQUALS, ARROW, EQUALS, PLUS, MINUS, STAR, SLASH,
  LESS_THAN, GREATER_THAN, BANG; delimiter types: LPAREN, RPAREN, LBRACE,
  RBRACE, LBRACKET, RBRACKET, COMMA, COLON, SEMICOLON, DOT.
- Alias resolution: definitions with `-> ALIAS` syntax emit the alias name.
- Line and column tracking for all tokens.
- `die` with a descriptive "LexerError" message on unexpected input.
- `t/00-load.t` — smoke test that the module loads and has a VERSION.
- `t/01-basic.t` — comprehensive test suite covering all keywords,
  identifiers, numbers, strings, operators, punctuation, arrow functions,
  composite expressions (if/else, function declarations, class declarations,
  method calls), whitespace, position tracking, and error handling.
- `BUILD` and `BUILD_windows` scripts.
