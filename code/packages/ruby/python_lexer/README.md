# Python Lexer

A Ruby gem that tokenizes Python source code using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding Python-specific tokenization rules, it loads the `python.tokens` grammar file and feeds it to the general-purpose lexer engine.

This demonstrates the core idea behind grammar-driven language tooling: the same engine can process any language, as long as you provide the right grammar file.

## How It Fits in the Stack

```
python.tokens (grammar file)
       |
       v
grammar_tools (parses .tokens into TokenGrammar)
       |
       v
lexer (GrammarLexer uses TokenGrammar to tokenize)
       |
       v
python_lexer (this gem -- thin wrapper providing Python API)
```

## Usage

```ruby
require "coding_adventures_python_lexer"

tokens = CodingAdventures::PythonLexer.tokenize("x = 1 + 2")
tokens.each { |t| puts t }
# Token(NAME, "x", 1:1)
# Token(EQUALS, "=", 1:3)
# Token(NUMBER, "1", 1:5)
# Token(PLUS, "+", 1:7)
# Token(NUMBER, "2", 1:9)
# Token(EOF, "", 1:10)
```

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
