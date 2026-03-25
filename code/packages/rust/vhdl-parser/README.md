# coding-adventures-vhdl-parser

A VHDL parser for the coding-adventures project. This crate parses VHDL (IEEE 1076-2008) source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `vhdl.grammar` file and feeds it, along with tokens from the `vhdl-lexer` crate, to the generic `GrammarParser`. The grammar file defines VHDL's syntactic structure in a declarative EBNF format covering entities, architectures, processes, signal/variable assignments, if/elsif/else, case/when, generate blocks, component instantiation, packages, and expressions with full operator precedence.

## How it fits in the stack

```
vhdl.tokens          (grammar file)
       |
       v
vhdl-lexer           (tokenizes VHDL source -> Vec<Token>)
       |
       v
vhdl.grammar         (grammar file)
       |
       v
parser               (GrammarParser: builds AST from tokens + grammar)
       |
       v
vhdl-parser          (THIS CRATE: wires everything together for VHDL)
```

## Usage

```rust
use coding_adventures_vhdl_parser::{create_vhdl_parser, parse_vhdl};

// Quick parsing -- returns a GrammarASTNode
let ast = parse_vhdl("entity empty is end entity empty;");
assert_eq!(ast.rule_name, "design_file");

// Or get the parser object for more control
let mut parser = create_vhdl_parser("entity adder is port (a, b : in bit; sum : out bit); end entity adder;");
let ast = parser.parse().expect("parse failed");
```

## Grammar rules

The VHDL grammar covers:

- **design_file** -- the top-level rule, a sequence of design units
- **entity_declaration** -- entity interfaces with ports and generics
- **architecture_body** -- entity implementations with declarations and concurrent statements
- **process_statement** -- sequential regions inside concurrent architectures
- **signal_assignment_concurrent** / **signal_assignment_seq** -- wire assignments (<=)
- **variable_assignment** -- immediate assignments (:=) inside processes
- **if_statement** -- if/elsif/else branching
- **case_statement** -- case/when multi-way branching
- **expression** -- full operator precedence from logical through unary and power
- **component_instantiation** -- structural composition of subcomponents
- **generate_statement** -- parameterized, replicated hardware
- **package_declaration** / **package_body** -- reusable declaration groups
