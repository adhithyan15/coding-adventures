# coding-adventures-algol-parser

A grammar-driven parser for **ALGOL 60** — the 1960 language whose grammar was
the first ever written in BNF (Backus-Naur Form), the notation still used in
every programming language specification and compiler textbook today.

## What Is ALGOL 60?

ALGOL 60 (ALGOrithmic Language) was designed by an international committee
including John Backus, Peter Naur, and Edsger Dijkstra. Its *Revised Report*
(1963) introduced formal grammar specification to the world. Key contributions:

- **BNF**: John Backus invented the notation; Peter Naur edited the report.
  Every language specification since (C, Java, Python, Go, Rust) uses
  derivatives of BNF.
- **Block structure**: `begin`...`end` scopes variables. The origin of every
  modern programming language's scope rules.
- **Dangling else resolved by grammar**: ALGOL resolves the ambiguous
  `if/then/else` at the grammar level by requiring the then-branch to be a
  non-conditional statement. C resolves it by convention (else binds nearest
  if) — a weaker solution.
- **Conditional expressions**: `if b then x else y` works in expressions,
  not just as statements. Cleaner than C's `b ? x : y` ternary.
- **Left-associative exponentiation**: `2^3^4 = (2^3)^4 = 4096` per the
  ALGOL 60 report. Unusual but correct per the specification.

## How It Fits the Stack

```
algol/algol60.grammar (grammar rules)  algol/algol60.tokens (token defs)
      │                                     │
      ▼                                     ▼
grammar_tools.compile_parser_grammar() algol-lexer
      │                                     │ list[Token]
      ▼                                     ▼
algol_parser._grammar.PARSER_GRAMMAR
      │
      ▼
GrammarParser ←── algol-parser (this package, no runtime file I/O)
      │
      ▼
ASTNode (generic parse tree)
```

This package depends on:
- **`coding-adventures-algol-lexer`**: Tokenizes ALGOL 60 source.
- **`coding-adventures-grammar-tools`**: Compiles `algol/algol60.grammar` into
  a native `ParserGrammar` module.
- **`coding-adventures-parser`**: The `GrammarParser` engine that runs the
  grammar against the token stream.

## Usage

```python
from algol_parser import parse_algol, create_algol_parser

# Simple parsing
ast = parse_algol("begin integer x; x := 42 end")
print(ast.rule_name)  # "program"

# Factory function (for more control)
parser = create_algol_parser("begin real pi; pi := 3.14 end")
ast = parser.parse()
```

## AST Structure

Parsing `begin integer x; x := 42 end` produces:

```
ASTNode(rule_name="program")
└── ASTNode(rule_name="block")
    ├── Token(BEGIN, 'begin')
    ├── ASTNode(rule_name="declaration")
    │   └── ASTNode(rule_name="type_decl")
    │       ├── Token(INTEGER, 'integer')
    │       └── Token(IDENT, 'x')
    ├── Token(SEMICOLON, ';')
    ├── ASTNode(rule_name="statement")
    │   └── ASTNode(rule_name="assign_stmt")
    │       ├── ASTNode(rule_name="left_part")
    │       │   ├── ASTNode(rule_name="variable")
    │       │   │   └── Token(IDENT, 'x')
    │       │   └── Token(ASSIGN, ':=')
    │       └── ASTNode(rule_name="expression")
    │           └── ASTNode(rule_name="arith_expr")
    │               └── ...
    └── Token(END, 'end')
```

## Grammar Rules

The parser covers the complete ALGOL 60 grammar:

| Category | Rules |
|----------|-------|
| Top level | `program`, `block` |
| Declarations | `type_decl`, `own_decl`, `own_array_decl`, `array_decl`, `switch_decl`, `procedure_decl` |
| Statements | `assign_stmt`, `cond_stmt`, `for_stmt`, `goto_stmt` (`goto` or `go to`), `proc_stmt`, `compound_stmt`, `dummy_stmt` |
| Arithmetic | `arith_expr`, `simple_arith`, `term`, `factor`, `primary` |
| Boolean | `bool_expr`, `simple_bool`, `implication`, `bool_term`, `bool_factor`, `bool_secondary`, `bool_primary`, `relation` |
| Designational | `desig_expr`, `simple_desig` |
| Variables | `variable`, `subscripts`, `proc_call`, `ident_list` |

Conditional arithmetic, Boolean, and designational expressions may nest in
either branch, so type-specific contexts such as array bounds/subscripts,
conditions, and `goto` targets accept the same nested conditional shape as
ordinary assignment expressions.

ALGOL dummy statements parse as zero-width `dummy_stmt` nodes at statement
boundaries, so empty `then`, `else`, and `do` bodies preserve semicolon
separator semantics instead of consuming the separator as part of the no-op.

Procedure declarations and calls accept the report-style omitted-parentheses
form for parameterless procedures as well as explicit empty parentheses, so
both `procedure p; p` and `procedure p(); p()` parse to zero-argument shapes.

## Development

```bash
pip install -e ".[dev]"
pytest
ruff check src/
```
