# Java Parser

A Ruby gem that parses Java source code into ASTs using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It loads `java/java<version>.grammar` and delegates all parsing to the general-purpose engine.

## Usage

```ruby
require "coding_adventures_java_parser"

# Default version (Java 21)
ast = CodingAdventures::JavaParser.parse("int x = 1 + 2;")
puts ast.rule_name  # "program"

# Specific version
ast = CodingAdventures::JavaParser.parse("int x = 1;", version: "8")

# Factory method for pipeline workflows
parser = CodingAdventures::JavaParser.create_parser("int x = 1;", version: "17")
```

## Supported Versions

- `"1.0"`, `"1.1"`, `"1.4"` -- early Java releases
- `"5"`, `"7"`, `"8"` -- Java SE 5 through 8
- `"10"`, `"14"`, `"17"`, `"21"` -- modern Java releases
- `nil` (default) -- uses Java 21 grammar

## Dependencies

- `coding_adventures_java_lexer` -- tokenizes Java source code
- `coding_adventures_grammar_tools` -- reads the `.grammar` grammar file
- `coding_adventures_parser` -- the grammar-driven parser engine

## Development

```bash
bundle install
bundle exec rake test
```
