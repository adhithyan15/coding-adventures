# Changelog — coding-adventures-json-value

## [0.1.0] - 2026-03-29

### Added

- Initial implementation of the JSON value evaluator.
- `M.null` — unique table sentinel representing JSON `null`; stores safely
  in Lua tables where `nil` would be silently dropped.
- `M.is_null(v)` — identity test for the null sentinel.
- `M.evaluate(ast)` — recursive AST walker dispatching on `rule_name`:
  - `"value"` → delegates to the single non-punctuation child
  - `"object"` → builds a Lua table with string keys from `pair` children
  - `"pair"` → extracts key (STRING) and evaluates value recursively
  - `"array"` → builds a Lua sequence table from `value` children
  - `"token"` (leaf) → decodes STRING/NUMBER/TRUE/FALSE/NULL tokens
- `M.from_string(json_str)` — convenience wrapper combining
  `json_parser.parse` + `M.evaluate` in a single call.
- `M.to_json(value, indent)` — serializer supporting all JSON types:
  - `nil` and `M.null` → `"null"`
  - booleans → `"true"` / `"false"`
  - integers → decimal without decimal point (e.g. `"42"`)
  - floats → `%.14g` format
  - NaN / ±Infinity → `"null"` (JSON has no representation)
  - strings → double-quoted with `"`, `\`, `\n`, `\t`, `\r`, `\f`, `\b`,
    and U+0000–U+001F control characters escaped
  - sequence tables → JSON array
  - other tables → JSON object with keys sorted alphabetically
  - `indent > 0` → pretty-print with `indent` spaces per nesting level
- `codepoint_to_utf8(cp)` — internal helper converting BMP code points
  (0–0xFFFF) to UTF-8 byte sequences, used by `\uXXXX` unescaping.
- `unescape_string(raw)` — internal helper stripping JSON string quotes and
  expanding all JSON escape sequences into their Lua equivalents.
- Full test suite (`tests/test_json_value.lua`) using busted:
  - Module surface (VERSION, null, is_null, evaluate, from_string, to_json)
  - Null sentinel identity, tostring, is_null variants
  - All scalar types: string, number (int, float, negative), boolean, null
  - String escapes: `\"`, `\\`, `\/`, `\n`, `\t`, `\r`, `\f`, `\b`,
    `\u0041` (ASCII), `\u00e9` (Latin-1), `\u4e2d` (CJK)
  - Objects: empty, single pair, multiple pairs, boolean/null values, nested
  - Arrays: empty, numbers, strings, mixed types, nested, array of objects
  - Complex mixed structure (realistic JSON document)
  - to_json scalars (nil, null, true, false, integers, floats, NaN, Infinity)
  - to_json string escaping (quotes, backslashes, control chars)
  - to_json arrays (compact, pretty, nested, with null sentinel)
  - to_json objects (compact, sorted keys, pretty, nested)
  - to_json mixed nested (object with array, array of objects)
  - Round-trip: from_string → to_json → from_string for all value types
  - Pretty-print produces valid JSON that can be parsed back
- `BUILD` and `BUILD_windows` with transitive dependency installation in
  leaf-to-root order: json_lexer → grammar_tools → lexer → state_machine →
  directed_graph → parser → json_parser → json_value.
- `required_capabilities.json` declaring no special capabilities.
- `README.md` with API reference, type-mapping table, usage examples,
  and null sentinel explanation.
