# ECMAScript ES1 Lexer

A Ruby gem that tokenizes ECMAScript 1 (ECMA-262, 1st Edition, 1997) source code using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding ES1-specific tokenization rules, it loads the `es1.tokens` grammar file and feeds it to the general-purpose lexer engine.

ES1 was the first standardized version of JavaScript. It includes `var` declarations, basic operators (`==`, `!=` but no `===`/`!==`), 23 keywords, and the foundational syntax that all later ECMAScript editions build upon.

## Usage

```ruby
require "coding_adventures_ecmascript_es1_lexer"

tokens = CodingAdventures::EcmascriptEs1Lexer.tokenize("var x = 1 + 2;")
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
