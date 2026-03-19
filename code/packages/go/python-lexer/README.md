# Python Lexer (Go Port)

**Layer 2-a of the computing stack** — tokenizes Python source code using a grammar-driven lexer.

## What does this package do?

This package wraps the grammar-based lexer with the `python.tokens` grammar file to tokenize Python source code. It reads the grammar definition at runtime, constructs a `GrammarLexer`, and produces a stream of tokens (keywords, identifiers, operators, literals, etc.) from raw Python source strings.

## Usage

```go
import (
	"fmt"
	pythonlexer "github.com/adhithyan15/coding-adventures/code/packages/go/python-lexer"
)

func main() {
	// One-shot tokenization
	tokens, err := pythonlexer.TokenizePython("print(\"hello\") if True else False")
	if err != nil {
		panic(err)
	}
	for _, tok := range tokens {
		fmt.Printf("%s: %q\n", tok.Type, tok.Value)
	}

	// Or create a reusable lexer instance
	lexer, err := pythonlexer.CreatePythonLexer("x = 1 + 2")
	if err != nil {
		panic(err)
	}
	tokens = lexer.Tokenize()
}
```

## Spec

See [02-lexer.md](../../../specs/02-lexer.md) for the full specification.
