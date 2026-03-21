# TypeScript Parser

A Ruby gem that parses TypeScript source code into ASTs using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `typescript.grammar` and delegates all parsing to the general-purpose engine.

## Usage

```ruby
require "coding_adventures_typescript_parser"

ast = CodingAdventures::TypescriptParser.parse("let x = 1 + 2;")
puts ast.rule_name  # "program"
```

## Dependencies

- `coding_adventures_typescript_lexer` -- tokenizes TypeScript source code
- `coding_adventures_grammar_tools` -- reads the `.grammar` grammar file
- `coding_adventures_parser` -- the grammar-driven parser engine

## Development

```bash
bundle install
bundle exec rake test
```
