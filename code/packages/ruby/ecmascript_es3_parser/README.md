# ECMAScript ES3 Parser

A Ruby gem that parses ECMAScript 3 (ECMA-262, 3rd Edition, 1999) source code into ASTs using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `es3.grammar` and delegates all parsing to the general-purpose engine.

ES3 extends ES1 with try/catch/finally/throw statements, strict equality operators (===, !==), the `instanceof` operator, and regex literals as primary expressions.

## Usage

```ruby
require "coding_adventures_ecmascript_es3_parser"

ast = CodingAdventures::EcmascriptEs3Parser.parse("try { x; } catch (e) { }")
puts ast.rule_name  # "program"
```

## Dependencies

- `coding_adventures_ecmascript_es3_lexer` -- tokenizes ES3 source code
- `coding_adventures_grammar_tools` -- reads the `.grammar` grammar file
- `coding_adventures_parser` -- the grammar-driven parser engine

## Development

```bash
bundle install
bundle exec rake test
```
