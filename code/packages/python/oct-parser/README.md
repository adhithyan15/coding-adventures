# oct-parser

Parses **Oct** source text into ASTs using the grammar-driven approach — a thin wrapper around `GrammarParser` that loads `oct.grammar`.

**Oct** is a small, statically-typed, 8-bit systems programming language designed to compile to the Intel 8008 microprocessor (1972). The name comes from *octet* — the networking term for exactly 8 bits, the native word size of the 8008 ALU.

## Overview

This package is part of the Oct compiler pipeline:

```
Oct source (.oct)
    ↓  [oct-lexer]        tokenise             ← produces tokens
    ↓  [oct-parser]       parse to AST         ← this package
    ↓  [oct-type-checker] type check
    ↓  [oct-ir-compiler]  lower to compiler_ir
    ↓  [ir-to-intel-8008-compiler] code generate
    ↓  [intel-8008-assembler] assemble to binary
    ↓  [intel-8008-packager]  produce Intel HEX
```

`oct-parser` occupies the second stage. It consumes the token stream from `oct-lexer` and produces a generic `ASTNode` tree using the EBNF rules in `oct.grammar`.

## Usage

```python
from oct_parser import parse_oct

ast = parse_oct("""
fn main() {
    let n: u8 = 0;
    while n != 255 {
        out(1, n);
        n = n + 1;
    }
    out(1, 255);
}
""")

print(ast.rule_name)  # "program"
```

For streaming (tokenize then parse separately):

```python
from oct_parser import create_oct_parser

parser = create_oct_parser("fn main() { let x: u8 = in(0); out(8, x); }")
ast = parser.parse()
```

## Grammar Summary

The grammar has two top-level declaration forms and eight statement kinds:

**Top-level**:
```
program     = { top_decl }
top_decl    = static_decl | fn_decl
static_decl = "static" NAME COLON type EQ expr SEMICOLON
fn_decl     = "fn" NAME LPAREN [ param_list ] RPAREN [ ARROW type ] block
```

**Statements**:
```
let_stmt    = "let" NAME COLON type EQ expr SEMICOLON
assign_stmt = NAME EQ expr SEMICOLON
return_stmt = "return" [ expr ] SEMICOLON
if_stmt     = "if" expr block [ "else" block ]
while_stmt  = "while" expr block
loop_stmt   = "loop" block
break_stmt  = "break" SEMICOLON
expr_stmt   = expr SEMICOLON
```

**Expression precedence** (lowest → highest):

| Level | Rule | Operators |
|-------|------|-----------|
| 1 | `or_expr` | `\|\|` |
| 2 | `and_expr` | `&&` |
| 3 | `eq_expr` | `==`, `!=` |
| 4 | `cmp_expr` | `<`, `>`, `<=`, `>=` |
| 5 | `add_expr` | `+`, `-` |
| 6 | `bitwise_expr` | `&`, `\|`, `^` |
| 7 | `unary_expr` | `!`, `~` |
| 8 | `primary` | literals, names, calls, parens |

**Intrinsic calls** (keyword-started, before `call_expr` in primary):

| Intrinsic | Signature | 8008 lowering |
|-----------|-----------|--------------|
| `in(PORT)` | → u8 | INP p |
| `out(PORT, val)` | void | MOV A, Rv; OUT p |
| `adc(a, b)` | → u8 | MOV A, Ra; ADC Rb |
| `sbb(a, b)` | → u8 | MOV A, Ra; SBB Rb |
| `rlc(a)` | → u8 | RLC |
| `rrc(a)` | → u8 | RRC |
| `ral(a)` | → u8 | RAL |
| `rar(a)` | → u8 | RAR |
| `carry()` | → bool | MVI A,0; ACI 0; MOV Rdst,A |
| `parity(a)` | → bool | ORA a; conditional materialise |

## AST Node Types

The root is always a `program` node. Key `rule_name` values you will encounter:

- `top_decl`, `static_decl`, `fn_decl`, `param_list`, `param`
- `block`, `stmt`
- `let_stmt`, `assign_stmt`, `return_stmt`, `if_stmt`, `while_stmt`, `loop_stmt`, `break_stmt`, `expr_stmt`
- `or_expr`, `and_expr`, `eq_expr`, `cmp_expr`, `add_expr`, `bitwise_expr`, `unary_expr`, `primary`
- `intrinsic_call`, `call_expr`, `arg_list`

## Dependencies

- `coding-adventures-oct-lexer` — tokenises Oct source
- `coding-adventures-parser` — `GrammarParser` engine
- `coding-adventures-grammar-tools` — `parse_parser_grammar`
- `coding-adventures-lexer`, `coding-adventures-directed-graph`, `coding-adventures-state-machine` — transitive deps
