# ECMAScript ES3 Parser (Go)

Parses ECMAScript 3 (1999) source code into ASTs using the grammar-driven parser engine. A thin wrapper that tokenizes with the ES3 lexer, loads `ecmascript/es3.grammar`, and delegates parsing to the generic `GrammarParser`.

## What ES3 Adds

- try/catch/finally/throw statements
- Strict equality (===, !==) in expressions
- instanceof operator
- Regex literals as primary expressions

## Usage

```go
import es3parser "github.com/adhithyan15/coding-adventures/code/packages/go/ecmascript-es3-parser"

ast, err := es3parser.ParseEs3("try { x === 1; } catch (e) {}")
```

## How It Works

1. Tokenizes source with the ES3 lexer (ecmascript-es3-lexer)
2. Reads `code/grammars/ecmascript/es3.grammar` at initialization
3. Uses the generic `GrammarParser` with PEG semantics and packrat memoization
