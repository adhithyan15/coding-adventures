# json-parser

Recursive-descent parser that converts `[Token]` into a `JsonValue` tree. Second stage of the JSON pipeline.

## Overview

`JsonParser` takes the flat token stream from `JsonLexer` and builds a structured `JsonValue`. It implements the JSON grammar from RFC 8259 using recursive descent.

```
json-value ← json-lexer ← json-parser  ← you are here
                               ↓
                       json-serializer  (JsonValue → String)
```

## Usage

```swift
import JsonParser
import JsonValue

let parser = JsonParser()

// Parse directly from a string (most convenient)
let value = try parser.parse("{\"name\": \"Alice\", \"scores\": [10, 20, 30]}")

// Or parse pre-lexed tokens
import JsonLexer
let lexer = JsonLexer()
let tokens = try lexer.tokenize("42")
let num = try parser.parseTokens(tokens)

// Access parsed data
if let name = value["name"]?.stringValue {
    print(name)  // "Alice"
}
if let scores = value["scores"]?.arrayValue {
    print(scores.count)  // 3
}
```

## Recursive descent

Each JSON grammar rule maps to a private function:

| Grammar rule | Function |
|---|---|
| `value → null \| bool \| number \| string \| array \| object` | `parseValue` |
| `array → '[' ... ']'` | `parseArray` |
| `object → '{' ... '}'` | `parseObject` |
| `pair → string ':' value` | `parsePair` |

## Error handling

`JsonParseError` is thrown for:
- Empty input
- Trailing commas (`[1, 2,]`, `{"a":1,}`)
- Missing `:` between object key and value
- Non-string object keys (`{42: "v"}`, `{true: "v"}`)
- Unterminated arrays or objects
- Extra tokens after the root value

## Strict JSON compliance

This parser is strict — it rejects constructs that are valid in JavaScript but not in JSON:
- Trailing commas in arrays and objects
- Single-quoted strings
- Comments (`//` or `/* */`)
- Bare identifiers (e.g., `undefined`)

## Part of the coding-adventures stack

This package is part of an educational monorepo exploring language implementations, data structures, and algorithms. All code uses Knuth-style literate programming.
