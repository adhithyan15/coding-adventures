# Changelog — CodingAdventures::JsonSerializer (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01 — initial release

### Added

- `encode($value, \%opts)` — serialize native Perl values to JSON with options:
  - `indent` — pretty-printing with configurable spaces per level
  - `sort_keys` — alphabetical key sorting for deterministic output (default on)
  - `allow_nan` — emit NaN/Infinity as quoted strings rather than null
  - `max_depth` — guard against stack overflow on deeply nested structures
- `decode($json_str, \%opts)` — parse JSON with pre-processing options:
  - `allow_comments` — strip `//` single-line and `/* */` multi-line comments (JSONC style)
  - `strict => 0` (default) — strip trailing commas from objects and arrays
  - `strict => 1` — forward raw input to the strict parser
- `validate($value, \%schema)` — validate a native Perl value against a JSON Schema subset:
  - Type keywords: `string`, `number`, `integer`, `boolean`, `null`, `object`, `array`
  - Object keywords: `properties`, `required`, `additional_properties`
  - Array keywords: `items`, `minItems`, `maxItems`
  - String keywords: `minLength`, `maxLength`, `pattern`
  - Number keywords: `minimum`, `maximum`
  - Cross-type keywords: `enum`
  - Returns all errors (not just the first) with dotted-path context
- `schema_encode($value, \%schema, \%opts)` — encode with schema-driven coercions:
  - Coerce numeric scalars to strings when schema specifies `type => 'string'`
  - Drop keys not in `properties` when `additional_properties => 0`
  - Recursively applied to nested objects and arrays
- `$NULL` and `is_null()` re-exported from `CodingAdventures::JsonValue`
- Full literate programming comments explaining JSON Schema concepts
- Test suite: `t/00-load.t` (smoke) and `t/01-basic.t` (comprehensive)
