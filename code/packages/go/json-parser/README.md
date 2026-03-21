# json-parser

A grammar-driven parser for JSON (RFC 8259) text. This package parses JSON input into an Abstract Syntax Tree (AST) using recursive descent with packrat memoization.

## How It Fits in the Stack

This package sits in the parsing layer of the grammar-driven compiler infrastructure:

```
json.tokens          json.grammar
     |                    |
grammar-tools        grammar-tools
     |                    |
   lexer               parser (generic engine)
     |                    |
json-lexer  --------> json-parser (this package)
  (tokenize)            (parse into AST)
```

The `json-parser` is a thin wrapper that:
1. Tokenizes JSON text using `json-lexer`
2. Loads the `json.grammar` file defining JSON's syntax rules
3. Delegates parsing to the generic `GrammarParser` engine

## Grammar Rules

JSON has exactly four grammar rules, making it the simplest practical grammar:

```
value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
object = LBRACE [ pair { COMMA pair } ] RBRACE ;
pair   = STRING COLON value ;
array  = LBRACKET [ value { COMMA value } ] RBRACKET ;
```

The grammar is recursive: `value` references `object` and `array`, which reference `value` again. This allows arbitrarily deep nesting.

## AST Structure

The parser produces an `ASTNode` tree that mirrors the grammar:

```
ParseJSON(`{"name": "Alice"}`)

  value
    object
      LBRACE("{")
      pair
        STRING("name")
        COLON(":")
        value
          STRING("Alice")
      RBRACE("}")
```

## Usage

```go
package main

import (
    "fmt"
    jsonparser "github.com/adhithyan15/coding-adventures/code/packages/go/json-parser"
)

func main() {
    // One-shot parsing
    ast, err := jsonparser.ParseJSON(`{"name": "Alice", "age": 30}`)
    if err != nil {
        panic(err)
    }
    fmt.Printf("Root rule: %s, children: %d\n", ast.RuleName, len(ast.Children))

    // Or create a reusable parser
    p, err := jsonparser.CreateJSONParser(`[1, 2, 3]`)
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
