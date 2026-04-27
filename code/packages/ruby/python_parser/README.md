# Python Parser

A Ruby gem that parses Python source code into Abstract Syntax Trees using versioned grammar-driven parser data.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads matching versioned `python{version}.grammar` and `python{version}.tokens` data, then uses the generic lexer and parser engines to transform Python source code into an AST.

This demonstrates the full grammar-driven pipeline: the same engines that could parse one language can parse any language, just by swapping the grammar files.

## How It Fits in the Stack

```text
python{version}.tokens + python{version}.grammar
           |                         |
           v                         v
      python_lexer              python_parser
       (tokens)                     (AST)
```

## Usage

```ruby
require "coding_adventures_python_parser"

ast = CodingAdventures::PythonParser.parse("x = 1 + 2")
# => ASTNode(rule_name: "file", ...)

ast = CodingAdventures::PythonParser.parse('print "hello"', version: "2.7")
# => ASTNode(rule_name: "file", ...)
```

## Dependencies

- `coding_adventures_grammar_tools` -- loads compiled versioned grammar data
- `coding_adventures_lexer` -- the grammar-driven lexer engine
- `coding_adventures_parser` -- the grammar-driven parser engine
- `coding_adventures_python_lexer` -- tokenizes Python source code

## Development

```bash
bundle install
bundle exec rake test
```
