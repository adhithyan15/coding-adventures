# json-lexer

A grammar-driven lexer for JSON (RFC 8259) text. This package tokenizes JSON input into a stream of typed tokens suitable for parsing.

## How It Fits in the Stack

This package sits in the tokenization layer of the grammar-driven compiler infrastructure:

```
json.tokens (grammar file)
        |
   grammar-tools (parses the grammar)
        |
      lexer (generic grammar-driven lexer engine)
        |
   json-lexer (this package -- thin wrapper)
        |
   json-parser (consumes the token stream)
```

The `json-lexer` is a thin wrapper that loads the `json.tokens` grammar file and delegates all tokenization work to the generic `GrammarLexer` engine. Unlike the Starlark lexer, JSON requires no indentation tracking, no keyword reclassification, and no reserved word checking.

## Token Types

| Token     | Description                          | Example        |
|-----------|--------------------------------------|----------------|
| STRING    | Double-quoted string with escapes    | `"hello\n"`    |
| NUMBER    | Integer, decimal, or scientific      | `42`, `-3.14`  |
| TRUE      | Boolean true literal                 | `true`         |
| FALSE     | Boolean false literal                | `false`        |
| NULL      | Null literal                         | `null`         |
| LBRACE    | Opening brace                        | `{`            |
| RBRACE    | Closing brace                        | `}`            |
| LBRACKET  | Opening bracket                      | `[`            |
| RBRACKET  | Closing bracket                      | `]`            |
| COLON     | Key-value separator                  | `:`            |
| COMMA     | Element separator                    | `,`            |
| EOF       | End of input                         | (implicit)     |

Whitespace (spaces, tabs, newlines, carriage returns) is silently consumed and produces no tokens.

## Usage

```go
package main

import (
    "fmt"
    jsonlexer "github.com/adhithyan15/coding-adventures/code/packages/go/json-lexer"
)

func main() {
    // One-shot tokenization
    tokens, err := jsonlexer.TokenizeJSON(`{"name": "Alice", "age": 30}`)
    if err != nil {
        panic(err)
    }
    for _, tok := range tokens {
        fmt.Printf("%s(%q)\n", tok.TypeName, tok.Value)
    }

    // Or create a reusable lexer
    lex, err := jsonlexer.CreateJSONLexer(`[1, 2, 3]`)
    if err != nil {
        panic(err)
    }
    tokens = lex.Tokenize()
}
```

## Running Tests

```bash
go test -v ./...
```
