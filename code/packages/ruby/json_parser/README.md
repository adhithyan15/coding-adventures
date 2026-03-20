# JSON Parser

A Ruby gem that parses JSON text into Abstract Syntax Trees using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `json.grammar` and `json.tokens`, then uses the generic lexer and parser engines to transform JSON text into an AST.

JSON (JavaScript Object Notation, RFC 8259) is defined by just four grammar rules, making it the simplest practical language for demonstrating the grammar-driven parser infrastructure.

This demonstrates the full grammar-driven pipeline: the same engines that could parse one language can parse any language, just by swapping the grammar files.

## How It Fits in the Stack

```
json.tokens + json.grammar (grammar files)
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
json_lexer       json_parser (this gem)
(tokens)         (AST)
```

## Usage

```ruby
require "coding_adventures_json_parser"

ast = CodingAdventures::JsonParser.parse('{"key": 42}')
# => ASTNode(rule_name: "value", children: [
#      ASTNode(rule_name: "object", children: [
#        Token(LBRACE, "{"),
#        ASTNode(rule_name: "pair", children: [
#          Token(STRING, "key"),
#          Token(COLON, ":"),
#          ASTNode(rule_name: "value", children: [
#            Token(NUMBER, "42")
#          ])
#        ]),
#        Token(RBRACE, "}")
#      ])
#    ])
```

## Dependencies

- `coding_adventures_grammar_tools` -- reads `.tokens` and `.grammar` files
- `coding_adventures_parser` -- the grammar-driven parser engine
- `coding_adventures_json_lexer` -- tokenizes JSON text

## Development

```bash
bundle install
bundle exec rake test
```
