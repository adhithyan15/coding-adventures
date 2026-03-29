# CodingAdventures::ExcelParser

A hand-written recursive-descent parser for Excel formulas in Perl.

## Overview

`CodingAdventures::ExcelParser` tokenizes an Excel formula string (using
`CodingAdventures::ExcelLexer`) and constructs an Abstract Syntax Tree (AST)
representing the formula's structure.

## Synopsis

```perl
use CodingAdventures::ExcelParser;

my $ast = CodingAdventures::ExcelParser->parse('=SUM(A1:B10)');
print $ast->rule_name;             # formula
print $ast->{body}->rule_name;     # call
print $ast->{body}{name}{value};   # sum
print scalar @{$ast->{body}{args}};# 1
print $ast->{body}{args}[0]->rule_name; # range

my $ast2 = CodingAdventures::ExcelParser->parse('=IF(A1>0,"pos","neg")');
print scalar @{$ast2->{body}{args}};  # 3
```

## Grammar

```
formula             = [ EQUALS ] expression ;
expression          = comparison_expr ;
comparison_expr     = concat_expr { comp_op concat_expr } ;
concat_expr         = additive_expr { AMP additive_expr } ;
additive_expr       = multiplicative_expr { (PLUS|MINUS) multiplicative_expr } ;
multiplicative_expr = power_expr { (STAR|SLASH) power_expr } ;
power_expr          = unary_expr { CARET unary_expr } ;
unary_expr          = { (PLUS|MINUS) } postfix_expr ;
postfix_expr        = primary { PERCENT } ;
primary             = LPAREN expression RPAREN | array_constant
                    | function_call | ref_prefix_expr
                    | cell_range | CELL | NAME
                    | NUMBER | STRING | BOOL | ERROR_CONSTANT ;
```

## AST node kinds

| Kind         | Fields                                          |
|--------------|-------------------------------------------------|
| `formula`    | `eq` (token or undef), `body` (ASTNode)         |
| `binop`      | `op` (token), `left`, `right` (ASTNodes)        |
| `unop`       | `op` (token), `operand` (ASTNode)               |
| `postfix`    | `op` (PERCENT token), `operand` (ASTNode)       |
| `call`       | `name` (token), `args` (arrayref)               |
| `range`      | `start_ref`, `end_ref` (ASTNodes)               |
| `ref_prefix` | `prefix` (token), `ref` (ASTNode or undef)      |
| `cell`       | `token`                                         |
| `number`     | `token`                                         |
| `string`     | `token`                                         |
| `bool`       | `token`                                         |
| `error`      | `token`                                         |
| `name`       | `token`                                         |
| `array`      | `rows` (arrayref of arrayrefs of ASTNodes)      |
| `group`      | `expr` (ASTNode)                                |

## Operator precedence

From lowest to highest:

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
cpanm --notest --quiet .
```

## Dependencies

- `CodingAdventures::ExcelLexer`
- `CodingAdventures::GrammarTools` (transitive)
- `CodingAdventures::Lexer` (transitive)

## Testing

```bash
prove -l -v t/
```

## License

MIT
