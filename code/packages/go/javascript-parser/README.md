# JavaScript Parser (Go)

Parses JavaScript source code into ASTs using the grammar-driven parser engine. A thin wrapper that loads `javascript.grammar` and delegates parsing to the generic `GrammarParser`.

## Usage

```go
import javascriptparser "github.com/adhithyan15/coding-adventures/code/packages/go/javascript-parser"

ast, err := javascriptparser.ParseJavascript("let x = 1 + 2;")
```
