# Changelog — coding-adventures-json-serializer (Lua)

## 0.1.0 — initial release

### Added

- `encode(value, opts)` — serialize native Lua values to JSON with options:
  - `indent` — pretty-printing with configurable spaces per level
  - `sort_keys` — alphabetical key sorting for deterministic output (default true)
  - `allow_nan` — emit NaN/Infinity as quoted strings rather than null
  - `max_depth` — guard against stack overflow on deeply nested structures
- `decode(json_str, opts)` — parse JSON with pre-processing options:
  - `allow_comments` — strip `//` single-line and `/* */` multi-line comments (JSONC style)
  - `strict = false` (default) — strip trailing commas from objects and arrays
  - `strict = true` — forward raw input to the strict parser
- `validate(value, schema)` — validate a native Lua value against a JSON Schema subset:
  - Type keywords: `string`, `number`, `integer`, `boolean`, `null`, `object`, `array`
  - Object keywords: `properties`, `required`, `additional_properties`
  - Array keywords: `items`, `minItems`, `maxItems`
  - String keywords: `minLength`, `maxLength`, `pattern`
  - Number keywords: `minimum`, `maximum`
  - Cross-type keywords: `enum`
  - Returns all errors (not just the first) with dotted-path context
- `schema_encode(value, schema)` — encode with schema-driven coercions:
  - Coerce `number` → `string` when schema specifies `type = "string"`
  - Drop keys not in `properties` when `additional_properties = false`
  - Recursively applied to nested objects and arrays
- Re-exports `null` sentinel and `is_null` from `json_value`
- Full literate programming comments explaining JSON Schema concepts
- Comprehensive busted test suite (95%+ coverage)
