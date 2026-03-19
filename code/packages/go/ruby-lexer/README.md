# Ruby Lexer (Go Port)

**Layer 2-b of the computing stack** — tokenizes Ruby source code using a grammar-driven lexer.

## What does this package do?

This package wraps the grammar-based lexer with the `ruby.tokens` grammar file to tokenize Ruby source code. It reads the grammar definition at runtime, constructs a `GrammarLexer`, and produces a stream of tokens (keywords, identifiers, operators, literals, etc.) from raw Ruby source strings.

## Usage

```go
import (
	"fmt"
	rubylexer "github.com/adhithyan15/coding-adventures/code/packages/go/ruby-lexer"
)

func main() {
	// One-shot tokenization
	tokens, err := rubylexer.TokenizeRuby("puts \"hello\" if true")
	if err != nil {
		panic(err)
	}
	for _, tok := range tokens {
		fmt.Printf("%s: %q\n", tok.Type, tok.Value)
	}

	// Or create a reusable lexer instance
	lexer, err := rubylexer.CreateRubyLexer("x = 1 + 2")
	if err != nil {
		panic(err)
	}
	tokens = lexer.Tokenize()
}
```

## Spec

See [02-lexer.md](../../../specs/02-lexer.md) for the full specification.
