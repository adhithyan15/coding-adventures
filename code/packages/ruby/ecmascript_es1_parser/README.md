# ECMAScript ES1 Parser

A Ruby gem that parses ECMAScript 1 (ECMA-262, 1st Edition, 1997) source code into ASTs using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `es1.grammar` and delegates all parsing to the general-purpose engine.

ES1 supports var declarations, function declarations/expressions, if/else, while, do-while, for, for-in, switch/case, with, break, continue, return, labelled statements, and the full expression precedence chain.

## Usage

```ruby
require "coding_adventures_ecmascript_es1_parser"

ast = CodingAdventures::EcmascriptEs1Parser.parse("var x = 1 + 2;")
puts ast.rule_name  # "program"
```

## Dependencies

- `coding_adventures_ecmascript_es1_lexer` -- tokenizes ES1 source code
- `coding_adventures_grammar_tools` -- reads the `.grammar` grammar file
- `coding_adventures_parser` -- the grammar-driven parser engine

## Development

```bash
bundle install
bundle exec rake test
```
