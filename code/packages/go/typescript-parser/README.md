# TypeScript Parser (Go)

Parses TypeScript source code into ASTs using the grammar-driven parser engine. A thin wrapper that loads `typescript.grammar` and delegates parsing to the generic `GrammarParser`.

## Usage

```go
import typescriptparser "github.com/adhithyan15/coding-adventures/code/packages/go/typescript-parser"

ast, err := typescriptparser.ParseTypescript("let x = 1 + 2;")
```
