# Verilog Parser

Parses Verilog HDL source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It loads `verilog.grammar` and delegates all parsing to the generic engine.

Verilog is a Hardware Description Language (HDL) used to design digital circuits. Unlike software languages, Verilog describes parallel hardware structures — modules, wires, registers, and gates.

## How It Fits in the Stack

```
Verilog source code
    |
    v
verilog_lexer.tokenize_verilog()   -- tokenizes using verilog.tokens
    |                                  (with optional preprocessor)
    v
verilog.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar()   -- parses the .grammar file
    |
    v
lang_parser.GrammarParser              -- generic parsing engine
    |
    v
verilog_parser.parse_verilog()         -- thin wrapper (this package)
    |
    v
ASTNode tree                           -- generic AST
```

## Usage

```python
from verilog_parser import parse_verilog

ast = parse_verilog('''
    module and_gate(input a, input b, output y);
        assign y = a & b;
    endmodule
''')
print(ast.rule_name)  # "source_text"
```

### With Preprocessing Disabled

```python
# Skip the preprocessor (no `define expansion)
ast = parse_verilog('module m; endmodule', preprocess=False)
```

### Using the Factory Function

```python
from verilog_parser import create_verilog_parser

parser = create_verilog_parser('module m; endmodule')
ast = parser.parse()
```

## Dependencies

- `coding-adventures-verilog-lexer` -- tokenizes Verilog source code (with preprocessor)
- `coding-adventures-parser` -- provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` -- parses `.grammar` files
