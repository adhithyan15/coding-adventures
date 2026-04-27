# Haskell Lexer

A Ruby gem that tokenizes Haskell source code using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding Haskell-specific tokenization rules, it loads the `haskell/haskell<version>.tokens` grammar file and feeds it to the general-purpose lexer engine.

## Usage

```ruby
require "coding_adventures_haskell_lexer"

# Default version (Haskell 21)
tokens = CodingAdventures::HaskellLexer.tokenize("int x = 1 + 2;")
tokens.each { |t| puts t }

# Specific version
tokens = CodingAdventures::HaskellLexer.tokenize("int x = 1;", version: "8")

# Factory method for pipeline workflows
lexer = CodingAdventures::HaskellLexer.create_lexer("int x = 1;", version: "17")
```

## Supported Versions

- `"1.0"`, `"1.1"`, `"1.4"` -- early Haskell releases
- `"5"`, `"7"`, `"8"` -- Haskell SE 5 through 8
- `"10"`, `"14"`, `"17"`, `"21"` -- modern Haskell releases
- `nil` (default) -- uses Haskell 21 grammar

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
