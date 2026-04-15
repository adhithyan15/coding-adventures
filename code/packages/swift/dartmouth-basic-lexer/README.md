# DartmouthBasicLexer (Swift)

A Dartmouth BASIC (1964) lexer that tokenizes BASIC source text into a stream of
typed `Token` values. A thin wrapper around the grammar-driven `GrammarLexer`
from the `Lexer` package, configured by `dartmouth_basic.tokens`.

## What is Dartmouth BASIC?

Dartmouth BASIC was invented in 1964 by John Kemeny and Thomas Kurtz to give
Dartmouth College students without programming backgrounds access to the campus
mainframe. It was the first interactive, time-shared programming language —
users typed at teletypes and got immediate responses.

The language is line-numbered, case-insensitive (teletypes had no lowercase),
and has only 17 statement types. Every variable is pre-initialised to 0.

## Usage

```swift
import DartmouthBasicLexer

let tokens = try DartmouthBasicLexer.tokenize("""
    10 LET X = 5
    20 PRINT X
    30 END
    """)

for token in tokens {
    print("\(token.type) \(token.value) (\(token.line):\(token.column))")
}
// LINE_NUM 10 (1:1)
// KEYWORD  LET (1:4)
// NAME     x   (1:8)
// EQ       =   (1:10)
// NUMBER   5   (1:12)
// NEWLINE  \n  (1:13)
// ...
```

## Token Types

| Type         | Example value | Description                              |
|--------------|---------------|------------------------------------------|
| `LINE_NUM`   | `"10"`        | Line label (first number on each line)   |
| `KEYWORD`    | `"LET"`       | Reserved word (uppercase)                |
| `NAME`       | `"x"`         | Variable name (lowercase after normalise)|
| `BUILTIN_FN` | `"sin"`       | Built-in function (lowercase)            |
| `USER_FN`    | `"fna"`       | User-defined function FNA–FNZ            |
| `NUMBER`     | `"3.14"`      | Numeric literal                          |
| `STRING`     | `"\"HELLO\""` | Double-quoted string literal             |
| `NEWLINE`    | `"\\n"`       | Line terminator (significant!)           |
| `EQ`         | `"="`         | Assignment and equality                  |
| `LT`, `GT`   | `"<"`, `">"`  | Comparison operators                     |
| `LE`, `GE`   | `"<="`, `">="` | Two-character comparison operators      |
| `NE`         | `"<>"`        | Not-equal operator                       |
| `PLUS`, `MINUS`, `STAR`, `SLASH`, `CARET` | arithmetic | Arithmetic |
| `LPAREN`, `RPAREN`, `COMMA`, `SEMICOLON` | punctuation | Punctuation |

## Post-processing

Two passes are applied after the raw GrammarLexer output:

1. **relabelLineNumbers** — The first `NUMBER` on each source line is promoted
   to `LINE_NUM`. This distinguishes line labels from arithmetic literals.

2. **suppressRemContent** — All tokens between `KEYWORD("REM")` and `NEWLINE`
   are removed. Comments do not reach the parser.

## Dependencies

- `GrammarTools` — parses `dartmouth_basic.tokens`
- `Lexer` — provides `GrammarLexer`

## Running tests

```bash
swift test --verbose
```

## Position in the stack

```
dartmouth_basic.tokens
        ↓
DartmouthBasicLexer  ← this package
        ↓
DartmouthBasicParser
        ↓
  compiler / VM
```
