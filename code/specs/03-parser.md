# 07 — Parser

## Overview

The parser takes a flat stream of tokens from the lexer and builds an Abstract Syntax Tree (AST) — a tree structure that represents the grammatical structure of the source code. The parser enforces the language's grammar rules: it knows that `1 + 2` is valid but `+ 1 2` is not.

The long-term goal is to make this parser grammar-driven: provide a grammar definition (like BNF or PEG), and the parser generates itself — like a simplified ANTLR or Bison.

This is Layer 7 of the computing stack. It depends on the lexer package.

## Layer Position

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → [YOU ARE HERE] → Compiler → VM
```

**Input from:** Lexer (provides the token stream).
**Output to:** Bytecode compiler (walks the AST to generate bytecode).

## Concepts

### Abstract Syntax Tree (AST)

The AST strips away syntactic sugar (parentheses, commas, semicolons) and represents only the meaningful structure:

```python
# Source: x = 1 + 2 * 3

# AST:
Assignment
├── target: Name("x")
└── value: BinaryOp
    ├── op: Add
    ├── left: Number(1)
    └── right: BinaryOp
        ├── op: Multiply
        ├── left: Number(2)
        └── right: Number(3)
```

Note: `2 * 3` is deeper in the tree than `1 +`, which means multiplication is evaluated first. The tree structure *encodes* operator precedence.

### Recursive Descent Parsing

The simplest parsing technique. Each grammar rule becomes a function:

```
expression  → term ((PLUS | MINUS) term)*
term        → factor ((STAR | SLASH) factor)*
factor      → NUMBER | NAME | LPAREN expression RPAREN
```

Maps directly to:

```python
def parse_expression(self):
    left = self.parse_term()
    while self.current_token.type in (PLUS, MINUS):
        op = self.consume()
        right = self.parse_term()
        left = BinaryOp(left, op, right)
    return left
```

### Operator Precedence

Precedence is encoded by the grammar rule nesting depth:
1. Deepest rules = highest precedence (evaluated first)
2. `factor` (numbers, names, parentheses) — highest
3. `term` (multiplication, division) — medium
4. `expression` (addition, subtraction) — lowest

### Grammar (MVP)

```
program     → statement*
statement   → assignment | expression_stmt | if_stmt | while_stmt | print_stmt
assignment  → NAME EQUALS expression NEWLINE
expression  → term ((PLUS | MINUS) term)*
term        → factor ((STAR | SLASH) factor)*
factor      → NUMBER | NAME | LPAREN expression RPAREN
if_stmt     → IF expression COLON NEWLINE statement*
while_stmt  → WHILE expression COLON NEWLINE statement*
print_stmt  → PRINT LPAREN expression RPAREN NEWLINE
```

## Public API

```python
# AST node types
@dataclass
class Number:
    value: int

@dataclass
class Name:
    id: str

@dataclass
class BinaryOp:
    left: Expression
    op: str          # "+", "-", "*", "/"
    right: Expression

@dataclass
class Assignment:
    target: Name
    value: Expression

@dataclass
class Print:
    value: Expression

@dataclass
class If:
    condition: Expression
    body: list[Statement]

@dataclass
class While:
    condition: Expression
    body: list[Statement]

@dataclass
class Program:
    statements: list[Statement]

# Type aliases
Expression = Number | Name | BinaryOp
Statement = Assignment | Print | If | While | ExpressionStatement

# Parser
class Parser:
    def __init__(self, tokens: list[Token]) -> None: ...

    def parse(self) -> Program: ...
        # Parse the token stream into an AST

    @property
    def errors(self) -> list[ParseError]: ...

@dataclass
class ParseError:
    message: str
    token: Token    # The token where the error was detected
```

## Data Flow

```
Input:  List of Token objects (from the lexer)
Output: AST (Program node) + list of errors
```

## Test Strategy

- Parse single number: `42` → Number(42)
- Parse single name: `x` → Name("x")
- Parse binary operation: `1 + 2` → BinaryOp(Number(1), "+", Number(2))
- Parse operator precedence: `1 + 2 * 3` → BinaryOp(Number(1), "+", BinaryOp(Number(2), "*", Number(3)))
- Parse parentheses override: `(1 + 2) * 3` → BinaryOp(BinaryOp(...), "*", Number(3))
- Parse assignment: `x = 1 + 2` → Assignment(Name("x"), BinaryOp(...))
- Parse if statement with body
- Parse while loop with body
- Parse print statement
- Verify error messages for: missing operand, unclosed parenthesis, unexpected token
- End-to-end with lexer: source string → tokens → AST

## Future Extensions

- **Grammar-driven parser generator**: Define grammar in BNF/PEG, generate the parser
- **Functions**: def, call, return, arguments
- **Comparison operators**: ==, !=, <, >, <=, >=
- **Boolean operators**: and, or, not
- **Error recovery**: Skip to next statement on error, continue parsing
- **Pretty printer**: AST → formatted source code
