# TypeScript Lexer (Go)

Tokenizes TypeScript source code using the grammar-driven lexer engine. A thin wrapper that loads `typescript.tokens` and delegates tokenization to the generic `GrammarLexer`.

## Usage

```go
import typescriptlexer "github.com/adhithyan15/coding-adventures/code/packages/go/typescript-lexer"

tokens, err := typescriptlexer.TokenizeTypescript("let x: number = 1 + 2;")
```
