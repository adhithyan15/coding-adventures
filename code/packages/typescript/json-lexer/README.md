# @coding-adventures/json-lexer

Tokenizes JSON text (RFC 8259) using the grammar-driven lexer infrastructure.

## What It Does

This package is a **thin wrapper** around the generic `grammarTokenize` function from `@coding-adventures/lexer`. It loads the `json.tokens` grammar file and delegates all tokenization work to the generic engine.

JSON is the simplest practical grammar in this project -- no keywords, no comments, no identifiers, no significant whitespace. This makes it an ideal first test case for the grammar-driven tokenization infrastructure.

## How It Fits in the Stack

```
json.tokens (grammar file)
    |
    v
grammar-tools (parses the grammar file)
    |
    v
lexer (generic grammarTokenize engine)
    |
    v
json-lexer (this package — thin wrapper)
    |
    v
json-parser (consumes the token stream)
```

## Usage

```typescript
import { tokenizeJSON } from "@coding-adventures/json-lexer";

const tokens = tokenizeJSON('{"name": "Alice", "age": 30}');

for (const token of tokens) {
  console.log(`${token.type}: ${token.value} (line ${token.line}, col ${token.column})`);
}

// Output:
// LBRACE: { (line 1, col 1)
// STRING: "name" (line 1, col 2)
// COLON: : (line 1, col 8)
// STRING: "Alice" (line 1, col 10)
// COMMA: , (line 1, col 17)
// STRING: "age" (line 1, col 19)
// COLON: : (line 1, col 24)
// NUMBER: 30 (line 1, col 26)
// RBRACE: } (line 1, col 28)
// EOF:  (line 1, col 29)
```

## Token Types

| Token     | Example   | Description                                |
|-----------|-----------|--------------------------------------------|
| STRING    | "hello"   | Double-quoted string with escape sequences |
| NUMBER    | -42, 3.14 | Integer, decimal, or scientific notation   |
| TRUE      | true      | Boolean true literal                       |
| FALSE     | false     | Boolean false literal                      |
| NULL      | null      | Null literal                               |
| LBRACE    | {         | Start of object                            |
| RBRACE    | }         | End of object                              |
| LBRACKET  | [         | Start of array                             |
| RBRACKET  | ]         | End of array                               |
| COLON     | :         | Key-value separator in objects             |
| COMMA     | ,         | Element separator                          |
| EOF       |           | End of input (always the last token)       |

## Running Tests

```bash
npm install
npm test
```
