# Starlark Parser

A Ruby gem that parses Starlark source code into Abstract Syntax Trees using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `starlark.grammar` and `starlark.tokens`, then uses the generic lexer and parser engines to transform Starlark source code into an AST.

Starlark is a deterministic subset of Python created by Google for Bazel BUILD files. It removes non-deterministic features (while loops, recursion, try/except, classes) to guarantee that build file evaluation always terminates.

This demonstrates the full grammar-driven pipeline: the same engines that could parse one language can parse any language, just by swapping the grammar files.

## How It Fits in the Stack

```
starlark.tokens + starlark.grammar (grammar files)
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
starlark_lexer   starlark_parser (this gem)
(tokens)         (AST)
```

## Usage

```ruby
require "coding_adventures_starlark_parser"

ast = CodingAdventures::StarlarkParser.parse("x = 1 + 2\n")
# => ASTNode(rule_name: "file", children: [
#      ASTNode(rule_name: "statement", children: [
#        ASTNode(rule_name: "simple_stmt", children: [
#          ASTNode(rule_name: "small_stmt", children: [
#            ASTNode(rule_name: "assign_stmt", children: [...])
#          ])
#        ])
#      ])
#    ])
```

## Dependencies

- `coding_adventures_grammar_tools` -- reads `.tokens` and `.grammar` files
- `coding_adventures_parser` -- the grammar-driven parser engine
- `coding_adventures_starlark_lexer` -- tokenizes Starlark source code

## Development

```bash
bundle install
bundle exec rake test
```
