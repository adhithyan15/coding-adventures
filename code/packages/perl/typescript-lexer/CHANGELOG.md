# Changelog — CodingAdventures::TypescriptLexer (Perl)

All notable changes to this package are documented here.

## [0.02] — 2026-04-05

### Added

- `tokenize($source, $version)` — optional `$version` parameter selects
  a versioned grammar file under `code/grammars/typescript/`.
- Valid version strings: `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`,
  `"ts5.0"`, `"ts5.8"`. Passing `undef` or `""` uses the generic
  `typescript.tokens` grammar (backward compatible).
- `_resolve_tokens_path($version)` — internal helper that maps version
  strings to grammar file paths.
- Per-version caches for grammar, compiled rules, skip rules, and keyword
  map (hashes keyed by version string instead of single package variables).
- Validation: unknown version strings raise a descriptive `die` immediately.
- Extended test suite: new version-aware subtests in `t/01-basic.t` covering
  all 6 recognized versions, cache consistency, and error cases.

### Changed

- `$VERSION` bumped from `0.01` to `0.02`.
- Package-level cache variables (`$_grammar`, `$_rules`, etc.) replaced by
  hash-based per-version caches (`%_grammar_cache`, etc.).
- `_grammar()` and `_build_rules()` now accept a `$version` argument.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::TypescriptLexer`.
- `tokenize($source)` — tokenizes a TypeScript string using rules compiled
  from the shared `typescript.tokens` grammar file.
- Grammar is read from `code/grammars/typescript.tokens` once and cached in
  package-level variables (`$_grammar`, `$_rules`, `$_skip_rules`).
- Path navigation uses `File::Basename::dirname` and `File::Spec::rel2abs`
  relative to `__FILE__`, climbing 5 directory levels to the repo root.
- Skip patterns (whitespace) are consumed silently; no WHITESPACE tokens
  are emitted.
- Full token set: all JavaScript tokens (NAME, NUMBER literal, STRING
  literal, LET, CONST, VAR, IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN,
  CLASS, IMPORT, EXPORT, FROM, AS, NEW, THIS, TYPEOF, INSTANCEOF, TRUE,
  FALSE, NULL, UNDEFINED, and all operators and delimiters) plus
  TypeScript-specific keyword tokens: INTERFACE, TYPE, ENUM, NAMESPACE,
  DECLARE, READONLY, PUBLIC, PRIVATE, PROTECTED, ABSTRACT, IMPLEMENTS,
  EXTENDS, KEYOF, INFER, NEVER, UNKNOWN, ANY, VOID, NUMBER (keyword),
  STRING (keyword), BOOLEAN, OBJECT, SYMBOL, BIGINT.
- Alias resolution: definitions with `-> ALIAS` syntax emit the alias name.
- Line and column tracking for all tokens.
- `die` with a descriptive "LexerError" message on unexpected input.
- `t/00-load.t` — smoke test that the module loads and has a VERSION.
- `t/01-basic.t` — comprehensive test suite covering: inherited JavaScript
  keywords, TypeScript-specific keywords, access modifiers, primitive type
  keywords, TypeScript constructs (type annotations, generics, interfaces,
  enums, abstract classes, implements/extends, keyof, as, declare,
  readonly), whitespace handling, position tracking, and error handling.
- `BUILD` and `BUILD_windows` scripts.
