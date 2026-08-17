# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `JsonParser.create_parser/0` now imports a pre-compiled grammar module (`CodingAdventures.JsonParser.Grammar`) instead of `File.read!`-ing `json.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).

## 0.1.0 — 2026-03-20

### Added
- `JsonParser.parse/1` — parse JSON source code into an AST
- `JsonParser.create_parser/0` — parse json.grammar
- Grammar caching via `persistent_term` for repeated use
- 21 tests covering primitives, objects, arrays, nested structures, RFC 8259 example, whitespace, and errors
