# Verilog Parser

A Ruby gem that parses Verilog HDL source code into ASTs using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `verilog.grammar` and delegates all parsing to the general-purpose engine. The result is an Abstract Syntax Tree that mirrors the structure defined in the grammar: modules, ports, assignments, always blocks, case statements, and the full expression hierarchy.

## Usage

```ruby
require "coding_adventures_verilog_parser"

ast = CodingAdventures::VerilogParser.parse("module top; endmodule")
puts ast.rule_name  # "source_text"
```

## Dependencies

- `coding_adventures_verilog_lexer` -- tokenizes Verilog HDL source code
- `coding_adventures_grammar_tools` -- reads the `.grammar` grammar file
- `coding_adventures_lexer` -- the general-purpose lexer engine
- `coding_adventures_parser` -- the grammar-driven parser engine

## Development

```bash
bundle install
bundle exec rake test
```
