# @coding-adventures/toml-lexer

Tokenizes TOML text (v1.0.0) using the grammar-driven lexer infrastructure.

## What It Does

This package is a **thin wrapper** around the generic `grammarTokenize` function from `@coding-adventures/lexer`. It loads the `toml.tokens` grammar file and delegates all tokenization work to the generic engine.

TOML is significantly more complex than JSON -- four string types, date/time literals, bare keys, comments, newline-sensitive syntax, and multiple integer/float formats. This makes it an excellent test of the grammar-driven tokenization infrastructure's ability to handle real-world configuration languages.

## How It Fits in the Stack

```
toml.tokens (grammar file)
    |
    v
grammar-tools (parses the grammar file)
    |
    v
lexer (generic grammarTokenize engine)
    |
    v
toml-lexer (this package -- thin wrapper)
    |
    v
toml-parser (consumes the token stream)
```

## Usage

```typescript
import { tokenizeTOML } from "@coding-adventures/toml-lexer";

const tokens = tokenizeTOML('[server]\nhost = "localhost"\nport = 8080');

for (const token of tokens) {
  console.log(`${token.type}: ${token.value} (line ${token.line}, col ${token.column})`);
}

// Output:
// LBRACKET: [ (line 1, col 1)
// BARE_KEY: server (line 1, col 2)
// RBRACKET: ] (line 1, col 8)
// NEWLINE: \n (line 1, col 9)
// BARE_KEY: host (line 2, col 1)
// EQUALS: = (line 2, col 6)
// BASIC_STRING: localhost (line 2, col 8)
// NEWLINE: \n (line 2, col 19)
// BARE_KEY: port (line 3, col 1)
// EQUALS: = (line 3, col 6)
// INTEGER: 8080 (line 3, col 8)
// EOF:  (line 3, col 12)
```

## Token Types

| Token              | Example             | Description                              |
|--------------------|---------------------|------------------------------------------|
| ML_BASIC_STRING    | """hello"""         | Triple-double-quoted, escapes allowed    |
| ML_LITERAL_STRING  | '''hello'''         | Triple-single-quoted, no escapes         |
| BASIC_STRING       | "hello"             | Double-quoted, escapes allowed           |
| LITERAL_STRING     | 'hello'             | Single-quoted, no escapes                |
| OFFSET_DATETIME    | 1979-05-27T07:32Z   | Date+time with timezone offset           |
| LOCAL_DATETIME     | 1979-05-27T07:32:00 | Date+time without timezone               |
| LOCAL_DATE         | 1979-05-27          | Date only                                |
| LOCAL_TIME         | 07:32:00            | Time only                                |
| FLOAT              | 3.14, 1e10, inf     | Decimal, scientific, or special float    |
| INTEGER            | 42, 0xFF, 0b1010    | Decimal, hex, octal, or binary integer   |
| TRUE               | true                | Boolean true literal                     |
| FALSE              | false               | Boolean false literal                    |
| BARE_KEY           | server              | Unquoted key name                        |
| EQUALS             | =                   | Key-value separator                      |
| DOT                | .                   | Dotted key separator                     |
| COMMA              | ,                   | Array/inline-table element separator     |
| LBRACKET           | [                   | Table header / array start               |
| RBRACKET           | ]                   | Table header / array end                 |
| LBRACE             | {                   | Inline table start                       |
| RBRACE             | }                   | Inline table end                         |
| NEWLINE            | \n                  | Line break (significant in TOML)         |
| EOF                |                     | End of input (always the last token)     |

## Running Tests

```bash
npm install
npm test
```
