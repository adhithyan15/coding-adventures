# Python Lexer

A Ruby gem that tokenizes Python source code using the grammar-driven lexer engine, with support for multiple Python versions.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding Python-specific tokenization rules, it loads versioned `python{version}.tokens` grammar files and feeds them to the general-purpose lexer engine.

This demonstrates the core idea behind grammar-driven language tooling: the same engine can process any language, as long as you provide the right grammar file.

## Supported Python Versions

- `"2.7"` -- Python 2.7 (final Python 2 release)
- `"3.0"` -- Python 3.0 (print as function, not keyword)
- `"3.6"` -- Python 3.6 (f-strings, underscores in numeric literals)
- `"3.8"` -- Python 3.8 (walrus operator `:=`)
- `"3.10"` -- Python 3.10 (soft keywords: `match`, `case`, `_`)
- `"3.12"` -- Python 3.12 (soft keyword: `type`) -- **default**

## How It Fits in the Stack

```
python{version}.tokens (versioned grammar files)
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

# Default version (3.12)
tokens = CodingAdventures::PythonLexer.tokenize("x = 1 + 2")
tokens.each { |t| puts t }

# Specify a version
tokens = CodingAdventures::PythonLexer.tokenize("x = 1 + 2", version: "3.8")

# Python 2.7
tokens = CodingAdventures::PythonLexer.tokenize("print 'hello'", version: "2.7")
```

## Dependencies

- `coding_adventures_grammar_tools` -- loads compiled versioned `.tokens` grammars
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
