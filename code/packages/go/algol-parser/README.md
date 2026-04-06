# algol-parser

A grammar-driven parser for **ALGOL 60** (ALGOrithmic Language, 1960), implemented in Go.

## What Is ALGOL 60?

ALGOL 60 was the first programming language whose syntax was formally specified using **BNF (Backus-Naur Form)** — the notation invented by John Backus and Peter Naur specifically to describe ALGOL's grammar. Today, every major language standard (C, Java, Python, Go, Rust) includes a formal grammar in BNF or a close variant.

The ALGOL 60 grammar is notable for:

- **Declaration-before-use**: all variable declarations must precede all statements in a block
- **Dangling else resolution**: handled by the grammar, not by convention
- **Call-by-name by default**: arguments are re-evaluated on each use (Jensen's device exploits this)
- **Left-associative exponentiation**: `2^3^4 = (2^3)^4 = 4096` (differs from math convention)
- **Conditional expressions**: `if b then x else y` can appear inside arithmetic expressions

## How This Package Works

This package parses ALGOL 60 source text into an AST using a two-stage pipeline:

1. **Lexing** (algol-lexer): source text → token stream
   - Strips whitespace and `comment...;` comments
   - Reclassifies keywords from identifiers
   - Handles multi-character operators (`:=`, `**`, `<=`, `>=`, `!=`)

2. **Parsing** (this package): token stream → AST
   - Applies `algol.grammar` rules via recursive descent
   - Uses packrat memoization for O(n) parsing time
   - Produces a tree of `ASTNode` values mirroring the grammar structure

## Usage

```go
import algolparser "github.com/adhithyan15/coding-adventures/code/packages/go/algol-parser"

// One-shot parsing
ast, err := algolparser.ParseAlgol("begin integer x; x := 42 end")
if err != nil {
    log.Fatal(err)
}
fmt.Println(ast.RuleName) // "program"

// Two-step API for more control
p, err := algolparser.CreateAlgolParser("begin real pi; pi := 3.14159 end")
if err != nil {
    log.Fatal(err)
}
ast, err = p.Parse()
```

## Grammar Entry Point

The root rule is `program`, which matches a single ALGOL block:

```
program = block ;
block   = BEGIN { declaration SEMICOLON } statement { SEMICOLON statement } END ;
```

The AST root node always has `RuleName == "program"`.

## Example AST

For `begin integer x; x := 42 end`:

```
program
  block
    BEGIN("begin")
    declaration
      type_decl
        type
          INTEGER("integer")
        ident_list
          IDENT("x")
    SEMICOLON(";")
    statement
      unlabeled_stmt
        assign_stmt
          left_part
            variable
              IDENT("x")
            ASSIGN(":=")
          expression
            arith_expr
              simple_arith
                term
                  factor
                    primary
                      INTEGER_LIT("42")
    END("end")
```

## Key Grammar Rules

| Rule | Description |
|------|-------------|
| `program` | Entry point: a single block |
| `block` | `begin` declarations statements `end` |
| `declaration` | `type_decl`, `array_decl`, `switch_decl`, `procedure_decl` |
| `type_decl` | `integer x, y` — variable declaration |
| `statement` | Assignment, goto, procedure call, conditional, for, compound, empty |
| `assign_stmt` | `x := expression` |
| `cond_stmt` | `if bool_expr then unlabeled_stmt [ else statement ]` |
| `for_stmt` | `for i := 1 step 1 until 10 do statement` |
| `arith_expr` | Arithmetic expression with operator precedence |
| `bool_expr` | Boolean expression (eqv, impl, or, and, not, relations) |

## Stack

This package depends on:
- `go/algol-lexer` — ALGOL 60 tokenizer
- `go/parser` — generic GrammarParser engine
- `go/grammar-tools` — parser grammar file parser
