# ECMAScript ES1 Parser (Go)

Parses ECMAScript 1 (1997) source code into ASTs using the grammar-driven parser engine. A thin wrapper that tokenizes with the ES1 lexer, loads `ecmascript/es1.grammar`, and delegates parsing to the generic `GrammarParser`.

## Usage

```go
import es1parser "github.com/adhithyan15/coding-adventures/code/packages/go/ecmascript-es1-parser"

ast, err := es1parser.ParseEs1("var x = 1 + 2;")
// ast.RuleName == "program"
```

## How It Works

1. Tokenizes source with the ES1 lexer (ecmascript-es1-lexer)
2. Reads `code/grammars/ecmascript/es1.grammar` at initialization
3. Uses the generic `GrammarParser` with PEG semantics and packrat memoization
