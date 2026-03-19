# TypeScript Lexer

A Ruby gem that tokenizes TypeScript source code using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding TypeScript-specific tokenization rules, it loads the `typescript.tokens` grammar file and feeds it to the general-purpose lexer engine.

## Usage

```ruby
require "coding_adventures_typescript_lexer"

tokens = CodingAdventures::TypescriptLexer.tokenize("let x: number = 1 + 2;")
tokens.each { |t| puts t }
```

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
