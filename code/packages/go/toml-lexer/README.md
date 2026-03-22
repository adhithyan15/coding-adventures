# toml-lexer

A grammar-driven lexer for TOML (v1.0.0) text. This package tokenizes TOML input into a stream of typed tokens suitable for parsing.

## How It Fits in the Stack

This package sits in the tokenization layer of the grammar-driven compiler infrastructure:

```
toml.tokens (grammar file)
        |
   grammar-tools (parses the grammar)
        |
      lexer (generic grammar-driven lexer engine)
        |
   toml-lexer (this package -- thin wrapper)
        |
   toml-parser (consumes the token stream)
```

The `toml-lexer` is a thin wrapper that loads the `toml.tokens` grammar file and delegates all tokenization work to the generic `GrammarLexer` engine. Unlike JSON, TOML is newline-sensitive — newlines delimit key-value pairs — so the lexer emits NEWLINE tokens.

## Token Types

| Token              | Description                                | Example                        |
|--------------------|--------------------------------------------|--------------------------------|
| ML_BASIC_STRING    | Triple-double-quoted multi-line string     | `"""hello\nworld"""`           |
| ML_LITERAL_STRING  | Triple-single-quoted multi-line string     | `'''hello\nworld'''`           |
| BASIC_STRING       | Double-quoted string (escapes raw)         | `"hello\n"`                    |
| LITERAL_STRING     | Single-quoted string (no escapes)          | `'C:\path'`                    |
| OFFSET_DATETIME    | Full datetime with timezone                | `1979-05-27T07:32:00Z`        |
| LOCAL_DATETIME     | Datetime without timezone                  | `1979-05-27T07:32:00`         |
| LOCAL_DATE         | Date only                                  | `1979-05-27`                   |
| LOCAL_TIME         | Time only                                  | `07:32:00`                     |
| FLOAT              | Floating-point, inf, nan                   | `3.14`, `inf`, `nan`           |
| INTEGER            | Decimal, hex, octal, binary                | `42`, `0xff`, `0o77`, `0b1010` |
| TRUE               | Boolean true                               | `true`                         |
| FALSE              | Boolean false                              | `false`                        |
| BARE_KEY           | Unquoted key name                          | `server`, `my-key`             |
| EQUALS             | Key-value separator                        | `=`                            |
| DOT                | Dotted key separator                       | `.`                            |
| COMMA              | Element separator                          | `,`                            |
| LBRACKET           | Opening bracket                            | `[`                            |
| RBRACKET           | Closing bracket                            | `]`                            |
| LBRACE             | Opening brace                              | `{`                            |
| RBRACE             | Closing brace                              | `}`                            |
| NEWLINE            | Line break (significant in TOML)           | `\n`                           |
| EOF                | End of input                               | (implicit)                     |

Comments (# to end of line) and horizontal whitespace (spaces, tabs) are silently consumed.

## Escape Handling

TOML uses `escapes: none` in its grammar file. The lexer strips quotes from strings but leaves escape sequences as raw text. This is because TOML has four string types with different escape semantics — the parser's semantic layer handles type-specific escape processing.

## Usage

```go
package main

import (
    "fmt"
    tomllexer "github.com/adhithyan15/coding-adventures/code/packages/go/toml-lexer"
)

func main() {
    // One-shot tokenization
    tokens, err := tomllexer.TokenizeTOML("[server]\nhost = \"localhost\"")
    if err != nil {
        panic(err)
    }
    for _, tok := range tokens {
        fmt.Printf("%s(%q)\n", tok.TypeName, tok.Value)
    }

    // Or create a reusable lexer
    lex, err := tomllexer.CreateTOMLLexer("name = \"TOML\"")
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
