# TET02 — Tetrad Parser Specification

## Overview

The Tetrad parser consumes the token stream produced by the lexer (spec TET01) and
builds an Abstract Syntax Tree (AST). The parser is a hand-written **Pratt parser**
(also called a top-down operator precedence parser) for expressions, combined with a
recursive descent parser for statements and declarations.

A Pratt parser assigns every token type two optional parsing functions:

- **Null denotation (NUD)**: how to parse a token that appears at the *start* of an
  expression (e.g., a literal, an identifier, a unary operator, an opening paren)
- **Left denotation (LED)**: how to parse a token that appears *in the middle* of an
  expression, with something to its left already parsed (e.g., a binary operator)

Each LED also carries a **binding power** (precedence level). When `parse_expr(min_bp)`
is called, it keeps consuming infix operators as long as their binding power exceeds
`min_bp`. This naturally implements precedence and associativity with no special cases.

Pratt parsers are the standard choice for expression parsing in modern language
implementations — they are used in V8, Clang, Rust's compiler, and Crafting Interpreters.
The recursive-descent wrapper for statements is universal.

---

## AST Node Types

Every node is a Python dataclass with a `line` and `column` field for error reporting.

### Program

```python
@dataclass
class Program:
    decls: list[FnDecl | GlobalDecl]
    line: int = 0
    column: int = 0
```

The root of every AST.

### Declarations

```python
@dataclass
class FnDecl:
    name: str
    params: list[str]
    param_types: list[str | None]   # parallel to params; None = no annotation
    return_type: str | None         # None = no return type annotation
    body: Block
    line: int
    column: int

@dataclass
class GlobalDecl:
    name: str
    declared_type: str | None       # None = no annotation
    value: Expr
    line: int
    column: int
```

`FnDecl` represents `fn name(params...) { body }`. `GlobalDecl` represents a top-level
`let name = expr;`.

### Statements

```python
@dataclass
class Block:
    stmts: list[Stmt]
    line: int
    column: int

@dataclass
class LetStmt:
    name: str
    declared_type: str | None       # None = no annotation
    value: Expr
    line: int
    column: int

@dataclass
class AssignStmt:
    name: str
    value: Expr
    line: int
    column: int

@dataclass
class IfStmt:
    condition: Expr
    then_block: Block
    else_block: Block | None
    line: int
    column: int

@dataclass
class WhileStmt:
    condition: Expr
    body: Block
    line: int
    column: int

@dataclass
class ReturnStmt:
    value: Expr | None
    line: int
    column: int

@dataclass
class ExprStmt:
    expr: Expr
    line: int
    column: int
```

`ExprStmt` covers `out(x);` and any other expression used as a statement.

### Expressions

```python
# Union type alias used throughout
Expr = (BinaryExpr | UnaryExpr | CallExpr | InExpr |
        OutExpr | NameExpr | IntLiteral | GroupExpr)

@dataclass
class IntLiteral:
    value: int        # 0–255 (range checked in compiler, not parser)
    line: int
    column: int

@dataclass
class NameExpr:
    name: str
    line: int
    column: int

@dataclass
class BinaryExpr:
    op: str           # '+', '-', '*', '/', '%', '&', '|', '^',
                      # '<<', '>>', '==', '!=', '<', '<=', '>', '>=',
                      # '&&', '||'
    left: Expr
    right: Expr
    line: int
    column: int

@dataclass
class UnaryExpr:
    op: str           # '!', '~', '-'
    operand: Expr
    line: int
    column: int

@dataclass
class CallExpr:
    name: str
    args: list[Expr]
    line: int
    column: int

@dataclass
class InExpr:
    """Represents in() — read from I/O port."""
    line: int
    column: int

@dataclass
class OutExpr:
    """Represents out(expr) — write to I/O port.
    Modelled as an expression so it can appear as ExprStmt.
    Result value is undefined (void-like).
    """
    value: Expr
    line: int
    column: int

@dataclass
class GroupExpr:
    expr: Expr
    line: int
    column: int
```

`GroupExpr` wraps a parenthesized expression. The compiler treats it as transparent.

---

## Binding Power Table

Pratt parsers assign a numeric binding power to each infix operator. A higher number
means the operator binds tighter (higher precedence). Right-associative operators use
`right_bp = left_bp - 1` so the right side re-enters the parser at a lower threshold.

| Operators | Left BP | Right BP | Associativity |
|---|---|---|---|
| `\|\|` | 10 | 10 | Left |
| `&&` | 20 | 20 | Left |
| `==`, `!=` | 30 | 30 | Left |
| `<`, `>`, `<=`, `>=` | 40 | 40 | Left |
| `\|` | 50 | 50 | Left |
| `^` | 60 | 60 | Left |
| `&` | 70 | 70 | Left |
| `<<`, `>>` | 80 | 80 | Left |
| `+`, `-` | 90 | 90 | Left |
| `*`, `/`, `%` | 100 | 100 | Left |

Prefix operators (`!`, `~`, unary `-`) have no binding power because they are handled
by NUD functions (they start an expression, not continue one).

---

## Pratt Parser Algorithm

```python
def parse_expr(min_bp: int = 0) -> Expr:
    # Step 1: call the NUD for the current token (left-hand side)
    token = advance()
    left = nud(token)

    # Step 2: while next token is an infix operator with bp > min_bp, extend
    while True:
        op = peek()
        bp = left_bp(op)
        if bp is None or bp <= min_bp:
            break
        advance()            # consume the operator
        left = led(op, left) # build a BinaryExpr using the right-bp threshold

    return left
```

### NUD handlers

| Token type | NUD action |
|---|---|
| `INT` | Return `IntLiteral(token.value)` |
| `HEX` | Return `IntLiteral(token.value)` |
| `IDENT` | If next is `LPAREN`: parse call. Else: return `NameExpr(token.value)` |
| `KW_IN` | Expect `LPAREN RPAREN`, return `InExpr()` |
| `KW_OUT` | Expect `LPAREN`, parse `expr`, expect `RPAREN`, return `OutExpr(expr)` |
| `MINUS` | Parse `unary_expr(bp=110)`, return `UnaryExpr('-', operand)` |
| `BANG` | Parse `unary_expr(bp=110)`, return `UnaryExpr('!', operand)` |
| `TILDE` | Parse `unary_expr(bp=110)`, return `UnaryExpr('~', operand)` |
| `LPAREN` | Parse `expr(0)`, expect `RPAREN`, return `GroupExpr(expr)` |

Unary operators use bp=110, which is above all binary operators, so `-a * b` parses as
`(-a) * b` (the negation binds to `a` alone before multiplication sees it).

### LED handlers

Every infix operator shares the same LED shape:

```python
def led_binary(op: str, left: Expr, right_bp: int) -> BinaryExpr:
    right = parse_expr(right_bp)
    return BinaryExpr(op=op, left=left, right=right)
```

The `right_bp` value is the operator's binding power (same as left BP for all
left-associative operators in Tetrad, since Tetrad has no right-associative binaries).

---

## Statement Parsing

Statements are parsed by recursive descent (not Pratt). The entry point is
`parse_stmt()` which dispatches on the current token:

```python
def parse_stmt() -> Stmt:
    tok = peek()
    if tok.type == KW_LET:    return parse_let_stmt()
    if tok.type == KW_IF:     return parse_if_stmt()
    if tok.type == KW_WHILE:  return parse_while_stmt()
    if tok.type == KW_RETURN: return parse_return_stmt()
    if tok.type == IDENT and peek_next().type == EQ:
        return parse_assign_stmt()
    return parse_expr_stmt()
```

### Let Statement

```
"let" NAME "=" expr ";"
```

```python
def parse_let_stmt() -> LetStmt:
    expect(KW_LET)
    name = expect(IDENT).value
    declared_type = None
    if peek().type == COLON:
        advance()
        declared_type = parse_type()   # currently: expect KW_U8, return "u8"
    expect(EQ)
    value = parse_expr()
    expect(SEMI)
    return LetStmt(name=name, declared_type=declared_type, value=value)
```

### Assign Statement

Disambiguation from an expression statement: a bare `NAME` followed by `=` (not `==`)
is an assignment. The parser peeks two tokens to distinguish.

```python
def parse_assign_stmt() -> AssignStmt:
    name = expect(IDENT).value
    expect(EQ)
    value = parse_expr()
    expect(SEMI)
    return AssignStmt(name=name, value=value)
```

### If Statement

```
"if" expr block [ "else" block ]
```

The `else` is optional. There is no `elif` keyword; chains are written as:

```tetrad
if a { ... } else { if b { ... } else { ... } }
```

```python
def parse_if_stmt() -> IfStmt:
    expect(KW_IF)
    condition = parse_expr()
    then_block = parse_block()
    else_block = None
    if peek().type == KW_ELSE:
        advance()
        else_block = parse_block()
    return IfStmt(condition=condition, then_block=then_block, else_block=else_block)
```

### While Statement

```
"while" expr block
```

```python
def parse_while_stmt() -> WhileStmt:
    expect(KW_WHILE)
    condition = parse_expr()
    body = parse_block()
    return WhileStmt(condition=condition, body=body)
```

### Return Statement

```
"return" [ expr ] ";"
```

```python
def parse_return_stmt() -> ReturnStmt:
    expect(KW_RETURN)
    value = None
    if peek().type != SEMI:
        value = parse_expr()
    expect(SEMI)
    return ReturnStmt(value=value)
```

### Expression Statement

```
expr ";"
```

Covers `out(n);`, function calls, and any expression used for side effects.

```python
def parse_expr_stmt() -> ExprStmt:
    expr = parse_expr()
    expect(SEMI)
    return ExprStmt(expr=expr)
```

### Block

```
"{" stmt* "}"
```

```python
def parse_block() -> Block:
    expect(LBRACE)
    stmts = []
    while peek().type not in (RBRACE, EOF):
        stmts.append(parse_stmt())
    expect(RBRACE)
    return Block(stmts=stmts)
```

---

## Declaration Parsing

```python
def parse_top_decl() -> FnDecl | GlobalDecl:
    tok = peek()
    if tok.type == KW_FN:
        return parse_fn_decl()
    if tok.type == KW_LET:
        return parse_global_decl()
    raise ParseError(f"expected fn or let at top level", tok)
```

### Function Declaration

```
"fn" NAME "(" params? ")" block
```

```python
def parse_fn_decl() -> FnDecl:
    expect(KW_FN)
    name = expect(IDENT).value
    expect(LPAREN)
    params = []
    param_types = []
    if peek().type != RPAREN:
        pname, ptype = parse_param()
        params.append(pname)
        param_types.append(ptype)
        while peek().type == COMMA:
            advance()
            pname, ptype = parse_param()
            params.append(pname)
            param_types.append(ptype)
    expect(RPAREN)
    return_type = None
    if peek().type == ARROW:
        advance()
        return_type = parse_type()
    body = parse_block()
    return FnDecl(name=name, params=params, param_types=param_types,
                  return_type=return_type, body=body)

def parse_param() -> tuple[str, str | None]:
    """Parse 'NAME' or 'NAME : type'."""
    name = expect(IDENT).value
    if peek().type == COLON:
        advance()
        return name, parse_type()
    return name, None

def parse_type() -> str:
    """Parse a type annotation. Currently only 'u8' is valid."""
    tok = expect(KW_U8)
    return "u8"
```

### Global Declaration

```
"let" NAME "=" expr ";"
```

Same syntax as a let-statement, but at the top level it becomes a `GlobalDecl`.

```python
def parse_global_decl() -> GlobalDecl:
    expect(KW_LET)
    name = expect(IDENT).value
    declared_type = None
    if peek().type == COLON:
        advance()
        declared_type = parse_type()
    expect(EQ)
    value = parse_expr()
    expect(SEMI)
    return GlobalDecl(name=name, declared_type=declared_type, value=value)
```

---

## Error Handling

The parser raises `ParseError` for:

| Condition | Message |
|---|---|
| Expected token not found | `expected X, got Y at line N col C` |
| Unexpected token at expression start | `unexpected token Y in expression at line N col C` |
| Unexpected token at top level | `expected fn or let at top level, got Y at line N col C` |
| Call with no closing paren | `unclosed argument list for call to 'name'` |
| `in` used without `()` | `in must be called as in(), not as a bare name` |
| `:` followed by unknown type name | `unknown type 'foo'; only 'u8' is valid` |
| `->` with no following type | `expected type after '->', got EOF` |

The parser does not attempt error recovery in v1. The first `ParseError` aborts parsing.

---

## Concrete Parse Example

### Source

```tetrad
fn multiply(a, b) {
    let result = 0;
    while b > 0 {
        result = result + a;
        b = b - 1;
    }
    return result;
}
```

### AST

```
Program
└── FnDecl name='multiply' params=['a', 'b']
    └── Block
        ├── LetStmt name='result'
        │   └── IntLiteral value=0
        ├── WhileStmt
        │   ├── condition: BinaryExpr op='>'
        │   │   ├── left: NameExpr name='b'
        │   │   └── right: IntLiteral value=0
        │   └── body: Block
        │       ├── AssignStmt name='result'
        │       │   └── BinaryExpr op='+'
        │       │       ├── left: NameExpr name='result'
        │       │       └── right: NameExpr name='a'
        │       └── AssignStmt name='b'
        │           └── BinaryExpr op='-'
        │               ├── left: NameExpr name='b'
        │               └── right: IntLiteral value=1
        └── ReturnStmt
            └── NameExpr name='result'
```

---

## Python Package

The parser lives in `code/packages/python/tetrad-parser/`.

Depends on `coding-adventures-tetrad-lexer`.

### Public API

```python
from tetrad_parser import parse, ParseError
from tetrad_parser.ast import (
    Program, FnDecl, GlobalDecl, Block,
    LetStmt, AssignStmt, IfStmt, WhileStmt, ReturnStmt, ExprStmt,
    IntLiteral, NameExpr, BinaryExpr, UnaryExpr,
    CallExpr, InExpr, OutExpr, GroupExpr,
)

# Parse a Tetrad source string into an AST.
# Calls tokenize() internally.
# Raises LexError or ParseError on invalid input.
def parse(source: str) -> Program: ...

class ParseError(Exception):
    def __init__(self, message: str, line: int, column: int): ...
```

---

## Test Strategy

### Expression precedence tests

Verify that operator precedence matches the binding power table:
- `1 + 2 * 3` → `BinaryExpr('+', 1, BinaryExpr('*', 2, 3))` (mul binds tighter)
- `1 * 2 + 3` → `BinaryExpr('+', BinaryExpr('*', 1, 2), 3)`
- `a || b && c` → `BinaryExpr('||', a, BinaryExpr('&&', b, c))` (and binds tighter)
- `~a + b` → `BinaryExpr('+', UnaryExpr('~', a), b)` (unary binds tightest)
- `(1 + 2) * 3` → `BinaryExpr('*', GroupExpr(BinaryExpr('+', 1, 2)), 3)`

### Statement parsing tests

- `let x = 42;` → `LetStmt(name='x', value=IntLiteral(42))`
- `x = x + 1;` → `AssignStmt(name='x', value=BinaryExpr('+', NameExpr('x'), IntLiteral(1)))`
- `if a > 0 { ... }` → `IfStmt` with no `else_block`
- `if a { ... } else { ... }` → `IfStmt` with `else_block`
- `while n > 0 { ... }` → `WhileStmt`
- `return;` → `ReturnStmt(value=None)`
- `return x + 1;` → `ReturnStmt` with `BinaryExpr`
- `out(42);` → `ExprStmt(OutExpr(IntLiteral(42)))`

### Call expression tests

- `add(1, 2)` → `CallExpr(name='add', args=[IntLiteral(1), IntLiteral(2)])`
- `f()` → `CallExpr(name='f', args=[])`

### I/O expression tests

- `in()` → `InExpr()`
- `out(x)` → `OutExpr(NameExpr('x'))`

### Declaration tests

- `fn f(a, b) { return a; }` → `FnDecl(name='f', params=['a','b'], body=Block(...))`
- Top-level `let x = 10;` → `GlobalDecl(name='x', value=IntLiteral(10))`

### Error tests

- Missing `{` in block → `ParseError`
- Extra `)` → `ParseError`
- `in` without `()` → `ParseError`
- Bare `=` without left side → `ParseError`

### End-to-end tests

Parse all five example programs from TET00 and verify the top-level structure.

### Coverage target

95%+ line coverage.

---

## Version History

| Version | Date | Description |
|---|---|---|
| 0.1.0 | 2026-04-20 | Initial specification |
