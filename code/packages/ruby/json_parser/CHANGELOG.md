# Changelog

All notable changes to `coding_adventures_json_parser` will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `parse` now loads the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`) instead of reading and parsing `json.grammar` from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

## [0.1.0] - 2026-03-20

### Added
- Initial release
- `CodingAdventures::JsonParser.parse(source)` method that parses JSON text into ASTs
- Loads `json.grammar` and delegates to `GrammarDrivenParser`
- Supports all JSON value types: strings, numbers, booleans (true/false), null
- Supports empty and non-empty objects with key-value pairs
- Supports empty and non-empty arrays with mixed-type elements
- Supports arbitrarily deep nested structures (objects in arrays, arrays in objects)
- Error handling for invalid JSON (trailing commas, missing colons, empty input)
- Full test suite with SimpleCov coverage >= 80%
