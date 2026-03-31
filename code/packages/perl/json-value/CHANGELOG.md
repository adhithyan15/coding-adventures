# Changelog — CodingAdventures::JsonValue

## [0.01] - 2026-03-29

### Added

- Initial implementation of the JSON value evaluator and serializer.
- `$NULL` — blessed sentinel of class `CodingAdventures::JsonValue::Null`
  representing JSON `null`; stores safely in Perl hashes and arrays where
  `undef` would be ambiguous.
- `is_null($v)` — returns 1 when `ref($v) eq 'CodingAdventures::JsonValue::Null'`.
- `evaluate($ast_node)` — recursive AST walker dispatching on `rule_name`:
  - `"value"` → delegates to the single non-punctuation child
  - `"object"` → builds a Perl hashref from `pair` children
  - `"pair"` → extracts the STRING key and evaluates the value recursively
  - `"array"` → builds a Perl arrayref from `value` children
  - `"token"` (leaf) → decodes STRING/NUMBER/TRUE/FALSE/NULL tokens
- `from_string($json_str)` — convenience wrapper combining
  `CodingAdventures::JsonParser->parse` + `evaluate()` in a single call.
- `to_json($value, $indent)` — serializer supporting all JSON types:
  - `undef` and `$NULL` → `"null"`
  - numbers → integer or float string (via `_looks_like_number` + `sprintf`)
  - strings → double-quoted with `"`, `\`, `\n`, `\t`, `\r`, `\f`, `\b`,
    and U+0000–U+001F control characters escaped
  - arrayrefs → JSON array
  - hashrefs → JSON object with keys sorted alphabetically
  - `$indent > 0` → pretty-print with `$indent` spaces per nesting level
- `_unescape_string($raw)` — internal helper stripping JSON string quotes
  and expanding all JSON escape sequences, including `\uXXXX` via
  `Encode::encode_utf8(chr(hex(...)))`.
- `_unescape_char($c)` — single-char dispatch table for escape sequences.
- `_looks_like_number($s)` — conservative regex to distinguish JSON number
  strings from JSON strings without depending on `Scalar::Util`.
- `_number_to_json($n)` — integer-vs-float formatter.
- `_string_to_json($s)` — escape all required characters.
- `_array_to_json($aref, $indent, $depth)` — array serializer.
- `_object_to_json($href, $indent, $depth)` — object serializer with
  sorted keys.
- Full test suite (`t/00-load.t`, `t/01-basic.t`) covering:
  - Module load and null sentinel identity and `is_null`
  - Scalar evaluation: string, empty string, integer, negative, float,
    true, false, null
  - String escape sequences: `\"`, `\\`, `\/`, `\n`, `\t`, `\r`, `\f`,
    `\b`, `\u0041` (ASCII), `\u00e9` (Latin-1), `\u4e2d` (CJK)
  - Objects: empty, single pair, multiple pairs, booleans, null values,
    nested objects
  - Arrays: empty, numbers, strings, mixed types, nested, array of objects
  - Complex realistic JSON document
  - to_json scalars (undef, null, integers, floats, strings)
  - to_json string escaping (quotes, backslashes, control chars)
  - to_json arrays (compact, pretty, with null, nested)
  - to_json objects (compact, sorted keys, pretty, nested)
  - to_json mixed nested (object with array, array of objects)
  - Round-trip: from_string → to_json → from_string for multiple types
  - Pretty-print produces valid JSON that can be parsed back
- `BUILD` and `BUILD_windows` with transitive dependency installation in
  leaf-to-root order: json-lexer → grammar-tools → lexer → state-machine →
  directed-graph → parser → json-parser → json-value.
- `Makefile.PL` and `cpanfile`.
- `required_capabilities.json` declaring no special capabilities.
- `README.md` with type-mapping table, API reference, usage examples,
  null sentinel explanation, and stack diagram.
