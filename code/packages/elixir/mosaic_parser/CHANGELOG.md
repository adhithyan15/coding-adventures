# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - Unreleased

### Fixed

- Eliminated runtime grammar loading: `create_parser/0` now imports a pre-compiled grammar module (`CodingAdventures.MosaicParser.Grammar`) instead of `File.read!`-ing `mosaic.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent). The `fix_grammar_for_packrat/1` reordering of `slot_type` and `property_value` is unchanged and still applied after loading the pre-compiled grammar.

## [0.1.0] - 2026-04-04

### Added

- `CodingAdventures.MosaicParser.parse/1` — two-stage pipeline (lex + parse)
  that turns Mosaic source into an `ASTNode` tree rooted at `"file"`
- `CodingAdventures.MosaicParser.create_parser/0` — parses `mosaic.grammar`
  into a `ParserGrammar` struct for introspection
- `:persistent_term` caching so the grammar is only parsed once per VM
- 20 unit tests covering: `create_parser/0` grammar introspection; minimal
  component parsing; slot declarations (single, multiple, with default values,
  keyword types); property assignments (NUMBER, COLOR_HEX, DIMENSION, STRING,
  keyword property names, slot reference values); nested nodes; slot references
  as children; `when` blocks; `each` blocks; error cases (empty input, unclosed
  brace, missing keyword, invalid character)
