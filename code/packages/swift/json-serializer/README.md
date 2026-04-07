# json-serializer

Converts `JsonValue` to a JSON string (compact or pretty-printed). Final stage of the JSON pipeline. Also provides deserialization as a convenience.

## Overview

`JsonSerializer` is a bidirectional JSON codec: it serializes `JsonValue` to text and deserializes text back to `JsonValue`.

```
json-value ← json-lexer ← json-parser ← json-serializer  ← you are here
```

## Usage

```swift
import JsonSerializer
import JsonValue

let s = JsonSerializer()

// Serialize to compact JSON
let compact = s.serialize(.object([
    (key: "name", value: .string("Alice")),
    (key: "age", value: .number(30)),
]))
// → {"name":"Alice","age":30}

// Serialize to pretty-printed JSON
let pretty = JsonSerializer(mode: .pretty)
print(pretty.serialize(.array([.number(1), .number(2), .number(3)])))
// [
//   1,
//   2,
//   3
// ]

// Deserialize
let value = try s.deserialize("{\"x\": 1}")
// → JsonValue.object([(key: "x", value: .number(1))])
```

## Output modes

| Mode | Description | Example |
|---|---|---|
| `.compact` (default) | No extra whitespace | `{"a":1,"b":[1,2]}` |
| `.pretty` | 2-space indented | multi-line, human readable |

## String escaping

The serializer produces valid JSON by escaping all characters required by RFC 8259:

| Input | Escaped |
|---|---|
| `"` | `\"` |
| `\` | `\\` |
| newline | `\n` |
| tab | `\t` |
| carriage return | `\r` |
| backspace (U+0008) | `\b` |
| form feed (U+000C) | `\f` |
| other control chars (U+0000–U+001F) | `\uXXXX` |

Unicode characters above U+001F pass through unescaped.

## Number formatting

Integer-valued doubles serialize without a decimal point: `42.0` → `"42"`. This matches JavaScript's `JSON.stringify` and Python's `json.dumps` behavior.

## Roundtrip guarantee

For any `JsonValue` `v`:
```swift
let s = JsonSerializer()
let v2 = try s.deserialize(s.serialize(v))
assert(v == v2)  // always true
```

## Part of the coding-adventures stack

This package is part of an educational monorepo exploring language implementations, data structures, and algorithms. All code uses Knuth-style literate programming.
