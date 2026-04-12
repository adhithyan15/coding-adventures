# Java Lexer

A Ruby gem that tokenizes Java source code using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding Java-specific tokenization rules, it loads the `java/java<version>.tokens` grammar file and feeds it to the general-purpose lexer engine.

## Usage

```ruby
require "coding_adventures_java_lexer"

# Default version (Java 21)
tokens = CodingAdventures::JavaLexer.tokenize("int x = 1 + 2;")
tokens.each { |t| puts t }

# Specific version
tokens = CodingAdventures::JavaLexer.tokenize("int x = 1;", version: "8")

# Factory method for pipeline workflows
lexer = CodingAdventures::JavaLexer.create_lexer("int x = 1;", version: "17")
```

## Supported Versions

- `"1.0"`, `"1.1"`, `"1.4"` -- early Java releases
- `"5"`, `"7"`, `"8"` -- Java SE 5 through 8
- `"10"`, `"14"`, `"17"`, `"21"` -- modern Java releases
- `nil` (default) -- uses Java 21 grammar

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
