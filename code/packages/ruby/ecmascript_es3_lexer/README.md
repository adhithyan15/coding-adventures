# ECMAScript ES3 Lexer

A Ruby gem that tokenizes ECMAScript 3 (ECMA-262, 3rd Edition, 1999) source code using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. It loads the `es3.tokens` grammar file and feeds it to the general-purpose lexer engine.

ES3 was the version that made JavaScript a real, complete language. It added strict equality (`===`, `!==`), structured error handling (`try`/`catch`/`finally`/`throw`), regular expression literals, and the `instanceof` operator.

## Usage

```ruby
require "coding_adventures_ecmascript_es3_lexer"

tokens = CodingAdventures::EcmascriptEs3Lexer.tokenize("x === 1")
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
