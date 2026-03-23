# VHDL Parser

A Ruby gem that parses VHDL source code into ASTs using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `vhdl.grammar` and delegates all parsing to the general-purpose engine. The result is an Abstract Syntax Tree that mirrors the structure defined in the grammar: entities, architectures, signal declarations, process statements, if/elsif/else chains, case statements, and the full expression hierarchy.

VHDL (VHSIC Hardware Description Language) takes a fundamentally different approach from Verilog. Where Verilog is implicit and concise (like C), VHDL is explicit and verbose (like Ada). Every signal must be declared with its type. Every entity must have a separate architecture. This verbosity catches errors at compile time that would be silent bugs in Verilog.

## Usage

```ruby
require "coding_adventures_vhdl_parser"

ast = CodingAdventures::VhdlParser.parse("entity empty is end entity empty;")
puts ast.rule_name  # "design_file"
```

## Dependencies

- `coding_adventures_vhdl_lexer` -- tokenizes VHDL source code (with case normalization)
- `coding_adventures_grammar_tools` -- reads the `.grammar` grammar file
- `coding_adventures_lexer` -- the general-purpose lexer engine
- `coding_adventures_parser` -- the grammar-driven parser engine

## Development

```bash
bundle install
bundle exec rake test
```
