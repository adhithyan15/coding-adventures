# Changelog

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `tokenize` now loads the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`) instead of reading and parsing `toml.tokens` from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

## [0.1.0] - 2026-03-21

### Added

- `CodingAdventures::TomlLexer.tokenize(source)` — tokenize TOML text
- Loads `toml.tokens` grammar file for TOML v1.0.0
- All 20 TOML token types supported
- Newline-sensitive tokenization (NEWLINE tokens emitted)
- `escapes: none` mode — quotes stripped, escapes preserved for semantic layer
