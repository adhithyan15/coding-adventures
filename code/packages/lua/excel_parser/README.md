# coding-adventures-excel-parser

A hand-written recursive-descent parser for Excel formulas in Lua.

## Overview

`coding_adventures.excel_parser` tokenizes an Excel formula string (using
`coding-adventures-excel-lexer`) and constructs an Abstract Syntax Tree (AST)
representing the formula's structure.

## Example

```lua
local excel_parser = require("coding_adventures.excel_parser")

local ast = excel_parser.parse("=SUM(A1:B10)")
-- ast.kind                  → "formula"
-- ast.body.kind             → "call"
-- ast.body.name.value       → "sum"
-- ast.body.args[1].kind     → "range"
-- ast.body.args[1].start_ref.kind → "cell"

local ast2 = excel_parser.parse("=IF(A1>0,\"pos\",\"neg\")")
-- ast2.body.kind            → "call"
-- #ast2.body.args           → 3
-- ast2.body.args[1].kind    → "binop"  (A1>0)
-- ast2.body.args[1].op.type → "GREATER_THAN"
```

## Grammar

```
formula    = [ EQUALS ] expression ;
expression = comparison_expr ;

comparison_expr     = concat_expr { comp_op concat_expr } ;
comp_op             = EQUALS | NOT_EQUALS | LESS_THAN | LESS_EQUALS
                    | GREATER_THAN | GREATER_EQUALS ;
concat_expr         = additive_expr { AMP additive_expr } ;
additive_expr       = multiplicative_expr { (PLUS | MINUS) multiplicative_expr } ;
multiplicative_expr = power_expr { (STAR | SLASH) power_expr } ;
power_expr          = unary_expr { CARET unary_expr } ;
unary_expr          = { (PLUS | MINUS) } postfix_expr ;
postfix_expr        = primary { PERCENT } ;

primary = LPAREN expression RPAREN
        | array_constant
        | function_call
        | ref_prefix_expr
        | cell_range | CELL
        | NAME | NUMBER | STRING | BOOL | ERROR_CONSTANT ;
```

## AST node kinds

| Kind         | Fields                                          |
|--------------|-------------------------------------------------|
| `formula`    | `eq` (EQUALS token or nil), `body` (expr node)  |
| `binop`      | `op` (token), `left`, `right`                   |
| `unop`       | `op` (token), `operand`                         |
| `postfix`    | `op` (PERCENT token), `operand`                 |
| `call`       | `name` (token), `args` (array of nodes/nil)     |
| `range`      | `start_ref`, `end_ref`                          |
| `ref_prefix` | `prefix` (REF_PREFIX token), `ref` (node/nil)   |
| `cell`       | `token`                                         |
| `number`     | `token`                                         |
| `string`     | `token`                                         |
| `bool`       | `token`                                         |
| `error`      | `token`                                         |
| `name`       | `token`                                         |
| `array`      | `rows` (array of arrays of nodes)               |
| `group`      | `expr`                                          |

## Operator precedence

From lowest to highest binding strength:

1. Comparison: `=` `<>` `<` `<=` `>` `>=`
2. Concatenation: `&`
3. Additive: `+` `-`
4. Multiplicative: `*` `/`
5. Power: `^`
6. Unary prefix: `+` `-`
7. Postfix: `%`
8. Primary: literals, references, function calls, `()`

## Installation

```bash
luarocks make --local coding-adventures-excel-parser-0.1.0-1.rockspec
```

## Dependencies

- `lua >= 5.4`
- `coding-adventures-excel-lexer`
- `coding-adventures-state-machine`
- `coding-adventures-directed-graph`
- `coding-adventures-grammar-tools`
- `coding-adventures-lexer`

## Testing

```bash
cd tests && busted . --verbose --pattern=test_
```

## License

MIT
