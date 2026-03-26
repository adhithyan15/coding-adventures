# coding-adventures-verilog-parser

A Verilog parser for the coding-adventures project. This crate parses Verilog HDL source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `verilog.grammar` file and feeds it, along with tokens from the `verilog-lexer` crate, to the generic `GrammarParser`. The grammar file defines Verilog's syntactic structure in a declarative EBNF format covering modules, ports, assignments, always blocks, case statements, generate blocks, and expressions with full operator precedence.

## How it fits in the stack

```
verilog.tokens       (grammar file)
       |
       v
verilog-lexer        (tokenizes Verilog source -> Vec<Token>)
       |
       v
verilog.grammar      (grammar file)
       |
       v
parser               (GrammarParser: builds AST from tokens + grammar)
       |
       v
verilog-parser       (THIS CRATE: wires everything together for Verilog)
```

## Usage

```rust
use coding_adventures_verilog_parser::{create_verilog_parser, parse_verilog};

// Quick parsing -- returns a GrammarASTNode
let ast = parse_verilog("module top; endmodule");
assert_eq!(ast.rule_name, "source_text");

// Or get the parser object for more control
let mut parser = create_verilog_parser("module adder(input a, input b, output sum); assign sum = a + b; endmodule");
let ast = parser.parse().expect("parse failed");
```

## Grammar rules

The Verilog grammar covers:

- **source_text** -- the top-level rule, a sequence of module declarations
- **module_declaration** -- modules with ports, parameters, and body items
- **continuous_assign** -- combinational logic via `assign` statements
- **always_construct** -- behavioral logic with sensitivity lists
- **case_statement** -- multi-way branching for instruction decoding / muxes
- **expression** -- full operator precedence from ternary down to primary atoms
- **module_instantiation** -- structural composition of submodules
- **generate_region** -- parameterized, replicated hardware generation
- **function_declaration** / **task_declaration** -- reusable combinational / procedural blocks
