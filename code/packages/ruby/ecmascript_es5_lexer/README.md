# ECMAScript ES5 Lexer

A Ruby gem that tokenizes ECMAScript 5 (ECMA-262, 5th Edition, 2009) source code using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. It loads the `es5.tokens` grammar file and feeds it to the general-purpose lexer engine.

ES5 landed a decade after ES3. The lexical changes are modest -- the main addition is the `debugger` keyword (promoted from future-reserved). The real innovations were strict mode semantics, native JSON support, and property descriptors, which are semantic rather than lexical.

## Usage

```ruby
require "coding_adventures_ecmascript_es5_lexer"

tokens = CodingAdventures::EcmascriptEs5Lexer.tokenize("debugger;")
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
