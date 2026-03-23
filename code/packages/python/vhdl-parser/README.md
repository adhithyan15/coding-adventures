# VHDL Parser

Parses VHDL source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It loads `vhdl.grammar` and delegates all parsing to the generic engine.

VHDL (VHSIC Hardware Description Language) is a Hardware Description Language designed by the US Department of Defense. Unlike Verilog which is terse and C-like, VHDL is verbose and Ada-like, with strong typing, explicit declarations, and case-insensitive identifiers.

## How It Fits in the Stack

```
VHDL source code
    |
    v
vhdl_lexer.tokenize_vhdl()           -- tokenizes using vhdl.tokens
    |                                    (with case normalization)
    v
vhdl.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar()  -- parses the .grammar file
    |
    v
lang_parser.GrammarParser             -- generic parsing engine
    |
    v
vhdl_parser.parse_vhdl()             -- thin wrapper (this package)
    |
    v
ASTNode tree                          -- generic AST
```

## Usage

```python
from vhdl_parser import parse_vhdl

ast = parse_vhdl('''
    entity and_gate is
        port(a, b : in std_logic; y : out std_logic);
    end entity and_gate;
''')
print(ast.rule_name)  # "design_file"
```

### Case Insensitivity

VHDL is case-insensitive. `ENTITY`, `Entity`, and `entity` are all treated identically:

```python
# These all produce the same AST:
ast1 = parse_vhdl('entity e is end entity e;')
ast2 = parse_vhdl('ENTITY e IS END ENTITY e;')
ast3 = parse_vhdl('Entity E Is End Entity E;')
```

### Using the Factory Function

```python
from vhdl_parser import create_vhdl_parser

parser = create_vhdl_parser('entity e is end entity e;')
ast = parser.parse()
```

## Dependencies

- `coding-adventures-vhdl-lexer` -- tokenizes VHDL source code (with case normalization)
- `coding-adventures-parser` -- provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` -- parses `.grammar` files
