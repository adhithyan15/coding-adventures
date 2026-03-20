# Starlark Lexer (Go)

**Layer 2-b of the computing stack** -- tokenizes Starlark source code using a grammar-driven lexer.

## What does this package do?

This package wraps the grammar-based lexer with the `starlark.tokens` grammar file to tokenize Starlark source code. Starlark is a deterministic subset of Python designed for configuration files, most notably used in Bazel BUILD files.

The lexer operates in **indentation mode**, meaning it tracks indentation levels and emits synthetic INDENT/DEDENT tokens to delimit blocks -- just like Python. It also:

- Recognizes all Starlark keywords (def, if, for, return, etc.) and emits them as KEYWORD tokens
- Panics on reserved keywords (class, while, import, etc.) that are valid Python but not valid Starlark
- Skips comments (# to end of line) and inline whitespace
- Handles multi-character operators (**,  //, ==, !=, <=, >=, +=, etc.)
- Supports all string literal forms (single, double, triple-quoted, with r/b prefixes)
- Tokenizes integer literals (decimal, hex, octal) and float literals
- Suppresses NEWLINE/INDENT/DEDENT inside brackets ((), [], {})

## How it fits in the stack

```
starlark.tokens grammar file
        |
        v
  grammar-tools      (parses the .tokens file)
        |
        v
  lexer (GrammarLexer)  (generic grammar-driven tokenizer)
        |
        v
  starlark-lexer     (THIS PACKAGE: thin wrapper)
        |
        v
  starlark-parser    (consumes the token stream)
```

## Usage

```go
import (
    "fmt"
    starlarklexer "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-lexer"
)

func main() {
    // One-shot tokenization
    tokens, err := starlarklexer.TokenizeStarlark(`
def greet(name):
    return "Hello, " + name
`)
    if err != nil {
        panic(err)
    }
    for _, tok := range tokens {
        fmt.Printf("%-12s %q\n", tok.TypeName, tok.Value)
    }

    // Or create a reusable lexer instance
    lex, err := starlarklexer.CreateStarlarkLexer("x = 1 + 2")
    if err != nil {
        panic(err)
    }
    tokens = lex.Tokenize()
}
```

## Spec

See [02-lexer.md](../../../specs/02-lexer.md) for the full specification.
