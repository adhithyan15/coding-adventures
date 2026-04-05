# ECMAScript ES5 Parser (Go)

Parses ECMAScript 5 (2009) source code into ASTs using the grammar-driven parser engine. A thin wrapper that tokenizes with the ES5 lexer, loads `ecmascript/es5.grammar`, and delegates parsing to the generic `GrammarParser`.

## What ES5 Adds

- `debugger` statement
- Getter/setter properties in object literals (`{ get x() {}, set x(v) {} }`)

## Usage

```go
import es5parser "github.com/adhithyan15/coding-adventures/code/packages/go/ecmascript-es5-parser"

ast, err := es5parser.ParseEs5("debugger;")
```

## How It Works

1. Tokenizes source with the ES5 lexer (ecmascript-es5-lexer)
2. Reads `code/grammars/ecmascript/es5.grammar` at initialization
3. Uses the generic `GrammarParser` with PEG semantics and packrat memoization
