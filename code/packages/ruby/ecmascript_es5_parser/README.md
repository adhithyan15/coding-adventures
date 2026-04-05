# ECMAScript ES5 Parser

A Ruby gem that parses ECMAScript 5 (ECMA-262, 5th Edition, 2009) source code into ASTs using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `es5.grammar` and delegates all parsing to the general-purpose engine.

ES5 extends ES3 with the `debugger` statement and getter/setter property definitions in object literals. The rest of the grammar is identical to ES3.

## Usage

```ruby
require "coding_adventures_ecmascript_es5_parser"

ast = CodingAdventures::EcmascriptEs5Parser.parse("debugger;")
puts ast.rule_name  # "program"
```

## Dependencies

- `coding_adventures_ecmascript_es5_lexer` -- tokenizes ES5 source code
- `coding_adventures_grammar_tools` -- reads the `.grammar` grammar file
- `coding_adventures_parser` -- the grammar-driven parser engine

## Development

```bash
bundle install
bundle exec rake test
```
