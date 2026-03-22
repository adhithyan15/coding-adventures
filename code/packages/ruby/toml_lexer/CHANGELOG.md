# Changelog

## [0.1.0] - 2026-03-21

### Added

- `CodingAdventures::TomlLexer.tokenize(source)` — tokenize TOML text
- Loads `toml.tokens` grammar file for TOML v1.0.0
- All 20 TOML token types supported
- Newline-sensitive tokenization (NEWLINE tokens emitted)
- `escapes: none` mode — quotes stripped, escapes preserved for semantic layer
