# Changelog — cli_builder (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `TokenClassifier` — classifies argv tokens into typed events
- `SpecLoader` — validates and normalizes JSON CLI specs
- `HelpGenerator` — generates formatted help text
- `FlagValidator` — validates flag constraints
- `Parser` — three-phase parsing (routing + scanning + validation)
- `parse_table`, `parse_string`, `parse` — public API
- Flag types: boolean, string, integer, float, enum, count
- Subcommand routing, positional arguments, help/version builtins
