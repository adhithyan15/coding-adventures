# Changelog

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `parse` now loads the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`) instead of reading and parsing `toml.grammar` from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

## [0.1.0] - 2026-03-21

### Added

- `CodingAdventures::TomlParser.parse(source)` — parse TOML text into AST
- Loads `toml.grammar` for TOML v1.0.0 syntax rules
- All 11 TOML grammar rules supported
- Syntax-level parsing (semantic validation in Python reference impl)
