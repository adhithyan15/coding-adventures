# toml-parser

A grammar-driven parser for TOML (v1.0.0) text. This package parses TOML input into an Abstract Syntax Tree (AST) using recursive descent with packrat memoization.

## How It Fits in the Stack

This package sits in the parsing layer of the grammar-driven compiler infrastructure:

```
toml.tokens          toml.grammar
     |                    |
grammar-tools        grammar-tools
     |                    |
   lexer               parser (generic engine)
     |                    |
toml-lexer  --------> toml-parser (this package)
  (tokenize)            (parse into AST)
```

The `toml-parser` is a thin wrapper that:
1. Tokenizes TOML text using `toml-lexer`
2. Loads the `toml.grammar` file defining TOML's syntax rules
3. Delegates parsing to the generic `GrammarParser` engine

## Grammar Rules

TOML has ~12 grammar rules — more than JSON (4 rules) but far fewer than CSS (36 rules):

```
document           = { NEWLINE | expression } ;
expression         = array_table_header | table_header | keyval ;
keyval             = key EQUALS value ;
key                = simple_key { DOT simple_key } ;
simple_key         = BARE_KEY | BASIC_STRING | LITERAL_STRING | TRUE | FALSE | INTEGER | FLOAT | ... ;
table_header       = LBRACKET key RBRACKET ;
array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET ;
value              = BASIC_STRING | ML_BASIC_STRING | ... | array | inline_table ;
array              = LBRACKET array_values RBRACKET ;
array_values       = { NEWLINE } [ value ... ] ;
inline_table       = LBRACE [ keyval { COMMA keyval } ] RBRACE ;
```

The grammar is context-free. Semantic constraints (key uniqueness, table path consistency, inline table immutability) are enforced in a post-parse validation pass.

## AST Structure

The parser produces an `ASTNode` tree that mirrors the grammar:

```
ParseTOML("[server]\nhost = \"localhost\"")

  document
    expression
      table_header
        LBRACKET("[")
        key
          simple_key
            BARE_KEY("server")
        RBRACKET("]")
    NEWLINE
    expression
      keyval
        key
          simple_key
            BARE_KEY("host")
        EQUALS("=")
        value
          BASIC_STRING("localhost")
```

## Usage

```go
package main

import (
    "fmt"
    tomlparser "github.com/adhithyan15/coding-adventures/code/packages/go/toml-parser"
)

func main() {
    // One-shot parsing
    ast, err := tomlparser.ParseTOML("[server]\nhost = \"localhost\"")
    if err != nil {
        panic(err)
    }
    fmt.Printf("Root rule: %s, children: %d\n", ast.RuleName, len(ast.Children))

    // Or create a reusable parser
    p, err := tomlparser.CreateTOMLParser("name = \"TOML\"")
    if err != nil {
        panic(err)
    }
    ast, err = p.Parse()
    if err != nil {
        panic(err)
    }
}
```

## Running Tests

```bash
go test -v ./...
```
