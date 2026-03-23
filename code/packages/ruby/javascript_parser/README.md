# JavaScript Parser

A Ruby gem that parses JavaScript source code into ASTs using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `javascript.grammar` and delegates all parsing to the general-purpose engine.

## Usage

```ruby
require "coding_adventures_javascript_parser"

ast = CodingAdventures::JavascriptParser.parse("let x = 1 + 2;")
puts ast.rule_name  # "program"
```

## Dependencies

- `coding_adventures_javascript_lexer` -- tokenizes JavaScript source code
- `coding_adventures_grammar_tools` -- reads the `.grammar` grammar file
- `coding_adventures_parser` -- the grammar-driven parser engine

## Development

```bash
bundle install
bundle exec rake test
```
