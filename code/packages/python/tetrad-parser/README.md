# tetrad-parser

Stage 2 of the [Tetrad](../../specs/TET00-tetrad-language.md) pipeline. Converts the flat token stream from `tetrad-lexer` into an Abstract Syntax Tree (AST) ready for type checking and compilation.

## Pipeline position

```
source text
    → [tetrad-lexer]        token stream
    → [tetrad-parser]       AST            ← you are here
    → [tetrad-type-checker] typed AST
    → [tetrad-compiler]     bytecode
    → [tetrad-vm]           execution
    → [tetrad-jit]          native code
```

## Parser architecture

The parser has two parts:

**Pratt parser** (expressions) — assigns every token type a *binding power*. The core loop keeps consuming operators as long as their binding power exceeds the current minimum. This cleanly handles precedence without special cases:

```
Precedence (higher = tighter):
  ||  →  10    &&  →  20    == !=  →  30    < > <= >=  →  40
  |   →  50    ^   →  60    &      →  70    << >>      →  80
  + - →  90    * / %  → 100    unary prefix  → 110
```

**Recursive descent** (statements and declarations) — dispatches on the leading token. No precedence needed here since each statement form has a unique starting keyword.

## Public API

```python
from tetrad_parser import parse, ParseError
from tetrad_parser.ast import Program, FnDecl, BinaryExpr, ...

ast = parse("fn add(a: u8, b: u8) -> u8 { return a + b; }")
```

### `parse(source: str) -> Program`

Lex and parse a Tetrad source string. Calls `tetrad_lexer.tokenize()` internally.

Raises `LexError` (from the lexer) or `ParseError` (syntax error).

### `ParseError`

Carries `.message`, `.line`, `.column`.

## AST node types (in `tetrad_parser.ast`)

| Node | Description |
|---|---|
| `Program` | Root node; list of top-level declarations |
| `FnDecl` | Function declaration with optional type annotations |
| `GlobalDecl` | Top-level `let` |
| `Block` | `{ stmt* }` |
| `LetStmt` | Local variable `let x: u8 = expr;` |
| `AssignStmt` | `x = expr;` |
| `IfStmt` | `if expr block [else block]` |
| `WhileStmt` | `while expr block` |
| `ReturnStmt` | `return [expr];` |
| `ExprStmt` | Expression used as statement |
| `IntLiteral` | Decimal or hex integer |
| `NameExpr` | Variable reference |
| `BinaryExpr` | Infix operation |
| `UnaryExpr` | Prefix `!`, `~`, `-` |
| `CallExpr` | Function call |
| `InExpr` | `in()` I/O read |
| `OutExpr` | `out(expr)` I/O write |
| `GroupExpr` | Parenthesized expression |

## Installation

```bash
pip install coding-adventures-tetrad-parser
```

## Spec

See [`code/specs/TET02-tetrad-parser.md`](../../specs/TET02-tetrad-parser.md).
