# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading. Previously `get_grammar/1` (via
  `create_lexer/1`) read `python<version>.tokens` from
  `code/grammars/python/` via `File.read!` at an absolute path that walks
  outside this package's own directory — this works in the monorepo but
  would raise a `File.Error` on first use after a published Hex package is
  installed, since `code/grammars/` is not part of the package. All 6
  supported versions (`2.7`, `3.0`, `3.6`, `3.8`, `3.10`, `3.12`) are now
  compiled ahead of time into `CodingAdventures.PythonLexer.Grammar.V*`
  submodules (via `grammar-tools compile-tokens`) and looked up through a
  `version => &Grammar.V*.token_grammar/0` map, mirroring the pattern
  already used by `verilog_lexer`. `:persistent_term` caching (via
  `tokenize/2`) is preserved. No versions required `--force`; all 6
  `.tokens` files validated cleanly. Public API (`tokenize/2`,
  `create_lexer/1`, `default_version/0`, `supported_versions/0`) is
  unchanged — note `create_lexer/1` (and thus `tokenize/2` for
  unrecognised version strings) still does not raise `ArgumentError` the
  way the other language ports do; that pre-existing lack of validation was
  out of scope for this fix and is preserved as-is.

## 0.1.0 — 2026-03-24

### Added
- `PythonLexer.tokenize/1` — tokenize Python source code from `python.tokens`
- `PythonLexer.create_lexer/0` — parse and return the shared Python token grammar
- Grammar caching via `persistent_term` for repeated calls
- Tests covering keywords, operators, delimiters, strings, values, positions, and errors
