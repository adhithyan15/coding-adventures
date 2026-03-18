# Python Parser

A Ruby gem that parses Python source code into Abstract Syntax Trees using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `python.grammar` and `python.tokens`, then uses the generic lexer and parser engines to transform Python source code into an AST.

This demonstrates the full grammar-driven pipeline: the same engines that could parse one language can parse any language, just by swapping the grammar files.

## How It Fits in the Stack

```
python.tokens + python.grammar (grammar files)
       |                |
       v                v
grammar_tools     grammar_tools
(TokenGrammar)   (ParserGrammar)
       |                |
       v                v
lexer              parser
(GrammarLexer)   (GrammarDrivenParser)
       |                |
       v                v
python_lexer     python_parser (this gem)
(tokens)         (AST)
```

## Usage

```ruby
require "coding_adventures_python_parser"

ast = CodingAdventures::PythonParser.parse("x = 1 + 2")
# => ASTNode(rule_name: "program", children: [
#      ASTNode(rule_name: "statement", children: [
#        ASTNode(rule_name: "assignment", children: [
#          Token(NAME, "x"), Token(EQUALS, "="),
#          ASTNode(rule_name: "expression", children: [...])
#        ])
#      ])
#    ])
```

## Dependencies

- `coding_adventures_grammar_tools` -- reads `.tokens` and `.grammar` files
- `coding_adventures_lexer` -- the grammar-driven lexer engine
- `coding_adventures_parser` -- the grammar-driven parser engine
- `coding_adventures_python_lexer` -- tokenizes Python source code

## Development

```bash
bundle install
bundle exec rake test
```
