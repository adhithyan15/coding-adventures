# json-lexer

Tokenizes JSON text into a flat `[Token]` array. First stage of the JSON parsing pipeline.

## Overview

`JsonLexer` converts raw JSON text into a sequence of typed tokens. This separates the "what are the atoms?" question from the "what do the atoms mean?" question handled by the parser.

```
json-value ← json-lexer  ← you are here
                 ↓
            json-parser  ([Token] → JsonValue)
                 ↓
         json-serializer  (JsonValue → String)
```

## Usage

```swift
import JsonLexer

let lexer = JsonLexer()
let tokens = try lexer.tokenize("{\"name\":\"Alice\",\"age\":30}")
// → [Token(.leftBrace, 0), Token(.stringLit("name"), 1), Token(.colon, 7), ...]
```

## Token types

| TokenKind | JSON input |
|---|---|
| `.leftBrace` | `{` |
| `.rightBrace` | `}` |
| `.leftBracket` | `[` |
| `.rightBracket` | `]` |
| `.colon` | `:` |
| `.comma` | `,` |
| `.stringLit(String)` | `"hello"` (escape sequences decoded) |
| `.numberLit(Double)` | `42`, `3.14`, `-1e5` |
| `.trueLit` | `true` |
| `.falseLit` | `false` |
| `.nullLit` | `null` |

## String escape sequences

All RFC 8259 escapes are decoded:

| Escape | Character |
|---|---|
| `\"` | double quote |
| `\\` | backslash |
| `\/` | solidus (forward slash) |
| `\b` | backspace (U+0008) |
| `\f` | form feed (U+000C) |
| `\n` | newline |
| `\r` | carriage return |
| `\t` | tab |
| `\uXXXX` | Unicode BMP code point |
| `\uXXXX\uXXXX` | surrogate pair (code points > U+FFFF) |

## Error handling

`JsonLexError` is thrown for:
- Unexpected characters (e.g., `@`, `#`)
- Unterminated string literals
- Invalid escape sequences
- Raw control characters in strings
- Leading zeros in numbers (`007`)
- Missing digits in number parts

## Part of the coding-adventures stack

This package is part of an educational monorepo exploring language implementations, data structures, and algorithms. All code uses Knuth-style literate programming.
