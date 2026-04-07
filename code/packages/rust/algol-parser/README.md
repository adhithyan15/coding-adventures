# ALGOL 60 Parser

A grammar-driven parser for [ALGOL 60](https://en.wikipedia.org/wiki/ALGOL_60) (ALGOrithmic Language, 1960).

## What it does

This crate parses ALGOL 60 source text into an Abstract Syntax Tree (AST). It does not hand-write parsing rules — instead, it loads the `algol.grammar` file and feeds it together with the token stream (from `algol-lexer`) to the generic `GrammarParser` from the `parser` crate.

## How it fits in the stack

```text
algol.tokens         (token grammar — for the lexer)
       |
       v
algol-lexer          (tokenizes source → Vec<Token>)
       |
       v
algol.grammar        (parser grammar — defines syntactic structure)
       |
       v
parser::GrammarParser (recursive descent + packrat memoization)
       |
       v
algol-parser         (this crate — thin glue layer)
       |
       v
GrammarASTNode tree  (ready for interpretation, compilation, or analysis)
```

## Grammar overview

The ALGOL 60 grammar (~30 rules) is structured in three major sections:

### Declarations

| Rule             | Description                                          |
|------------------|------------------------------------------------------|
| `declaration`    | type_decl \| array_decl \| switch_decl \| procedure_decl |
| `type_decl`      | `integer x, y, z` — declares named variables        |
| `array_decl`     | `array A[1:10]` — dynamically-sized arrays           |
| `switch_decl`    | `switch s := label1, label2` — computed goto table  |
| `procedure_decl` | Full procedure with parameters and body              |

### Statements

| Rule             | Description                                          |
|------------------|------------------------------------------------------|
| `assign_stmt`    | `x := expr` — assignment (`:=` not `=`)             |
| `cond_stmt`      | `if bool then stmt [else stmt]`                      |
| `for_stmt`       | `for i := 1 step 1 until 10 do stmt`                |
| `goto_stmt`      | `goto label` — unconditional jump                    |
| `proc_stmt`      | `print(x)` — procedure call as statement             |
| `compound_stmt`  | `begin stmt ; stmt end` — grouped statements         |

### Expressions

| Rule             | Description                                          |
|------------------|------------------------------------------------------|
| `arith_expr`     | Conditional: `if b then x else y`, or `simple_arith` |
| `simple_arith`   | Addition/subtraction with optional leading sign      |
| `term`           | Multiplication, division, `div`, `mod`              |
| `factor`         | Exponentiation: `x ** n` or `x ^ n`                 |
| `primary`        | Literal, variable, proc_call, parenthesized expr    |
| `bool_expr`      | Conditional or `simple_bool`                         |
| `simple_bool`    | `eqv` chain → `impl` chain → `or` chain → `and` chain → `not` |
| `relation`       | `x < y`, `x = y`, `x >= y`, etc.                   |

## Historical context: why ALGOL 60 matters

ALGOL 60 introduced concepts that every modern language uses:

- **BNF notation** — the ALGOL 60 report was the first formal grammar specification. Every modern language spec (from C to Rust) uses BNF or a variant.
- **Block structure** — nested scopes with local variables. Before ALGOL, variables were global. ALGOL made scope explicit and lexical.
- **Recursion** — ALGOL 60 was the first widely-used language to support recursive procedure calls.
- **Call stack** — the runtime mechanism for recursion (stack frames with return addresses and locals) was invented to implement ALGOL.
- **Dangling-else resolution by grammar** — ALGOL resolves the ambiguity by requiring `begin...end` around nested conditionals. This is cleaner than C's "bind to nearest if" convention.

## Usage

```rust
use coding_adventures_algol_parser::parse_algol;

let ast = parse_algol("begin integer x; x := 42 end");
assert_eq!(ast.rule_name, "program");
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_algol_parser::create_algol_parser;

let mut parser = create_algol_parser("begin integer x; x := 0 end");
let ast = parser.parse().expect("parse failed");
println!("{:?}", ast.rule_name);
```

## Running tests

```bash
cargo test -p coding-adventures-algol-parser -- --nocapture
```
