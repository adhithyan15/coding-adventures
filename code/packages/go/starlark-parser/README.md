# Starlark Parser (Go)

**Layer 2-c of the computing stack** -- parses Starlark source code into an Abstract Syntax Tree using a grammar-driven parser.

## What does this package do?

This package wraps the grammar-based parser with the `starlark.grammar` grammar file to parse Starlark source code into a generic AST. It integrates with the starlark-lexer package for tokenization, then applies the parser grammar to produce a tree of `ASTNode` objects.

Starlark is a deterministic subset of Python designed for configuration files, most notably used in Bazel BUILD files. The parser handles the full Starlark syntax:

- **Statements**: assignment, return, break, continue, pass, load
- **Compound statements**: if/elif/else, for loops, function definitions (def)
- **Expressions**: full operator precedence chain (lambda, ternary, boolean, comparison, bitwise, arithmetic, unary, power)
- **Data structures**: list literals, dict literals, tuple literals, comprehensions
- **Function calls**: positional args, keyword args, *args, **kwargs
- **String concatenation**: adjacent string literals are parsed together

## How it fits in the stack

```
Source Code
    |
    v
starlark-lexer   (tokenizes, handles indentation)
    |
    v
Token Stream (with INDENT/DEDENT/NEWLINE)
    |
    v
starlark-parser  (THIS PACKAGE: grammar-driven parsing)
    |
    v
AST (tree of ASTNode objects)
```

## Usage

```go
import (
    "fmt"
    starlarkparser "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-parser"
)

func main() {
    // One-shot parsing
    ast, err := starlarkparser.ParseStarlark(`
cc_library(
    name = "mylib",
    srcs = ["mylib.cc"],
    deps = ["//base:logging"],
)
`)
    if err != nil {
        panic(err)
    }
    fmt.Printf("Root rule: %s, children: %d\n", ast.RuleName, len(ast.Children))

    // Or create a reusable parser instance
    p, err := starlarkparser.CreateStarlarkParser("x = 1 + 2\n")
    if err != nil {
        panic(err)
    }
    ast, err = p.Parse()
}
```

## Spec

See [03-parser.md](../../../specs/03-parser.md) for the full specification.
