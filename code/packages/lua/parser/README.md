# parser

Recursive descent parser building Abstract Syntax Trees from token streams.

## Layer 3

This package is part of Layer 3 of the coding-adventures computing stack.
Ported from the Go implementation at `code/packages/go/parser/`.

## Two Parsing Modes

### Hand-written Parser

A classic recursive descent parser with explicit precedence climbing. Recognises
a small language of expressions, assignments, and statements.

```lua
local parser = require("coding_adventures.parser")

local tokens = {
    { type = parser.TOKEN_NUMBER, value = "1", line = 1, column = 1 },
    { type = parser.TOKEN_PLUS,   value = "+", line = 1, column = 3 },
    { type = parser.TOKEN_NUMBER, value = "2", line = 1, column = 5 },
    { type = parser.TOKEN_EOF,    value = "",  line = 1, column = 6 },
}

local p = parser.Parser.new(tokens)
local program = p:parse()
-- program.statements[1].expression is a BinaryOp with op="+"
```

### Grammar-driven Parser

An interpreter that reads grammar rules (from grammar-tools) and parses any
token stream according to those rules. Uses packrat memoization for efficient
backtracking.

```lua
local parser = require("coding_adventures.parser")

-- Grammar elements are tables with a `kind` field
local g = {
    rules = {
        {
            name = "expr",
            body = { kind = "rule_reference", name = "NUMBER", is_token = true },
        },
    },
}

local tokens = {
    { type = parser.TOKEN_NUMBER, value = "42", line = 1, column = 1 },
    { type = parser.TOKEN_EOF,    value = "",   line = 1, column = 3 },
}

local p = parser.GrammarParser.new(tokens, g)
local ast, err = p:parse()
-- ast.rule_name == "expr", ast.children[1] is the NUMBER token
```

## AST Node Types

| Type           | Fields                    | Classification |
|----------------|---------------------------|----------------|
| NumberLiteral  | value (number)            | Expression     |
| StringLiteral  | value (string)            | Expression     |
| NameNode       | name (string)             | Expression     |
| BinaryOp       | left, op, right           | Expression     |
| Assignment     | target (NameNode), value  | Statement      |
| ExpressionStmt | expression                | Statement      |
| Program        | statements (array)        | Top-level      |
| ASTNode        | rule_name, children       | Grammar-driven |

## Dependencies

- grammar-tools (grammar element types for grammar-driven parser)
- lexer (token type constants and token format)

## Development

```bash
# Run tests
cd tests && busted . --verbose --pattern=test_
```
