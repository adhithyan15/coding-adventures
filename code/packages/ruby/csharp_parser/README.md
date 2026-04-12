# C# Parser

A Ruby gem that parses C# source code into Abstract Syntax Trees (ASTs) using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. It operates as a two-step pipeline:

1. **Lex**: `coding_adventures_csharp_lexer` loads `csharp/csharp<version>.tokens` and tokenizes the source into a flat list of `Token` objects.
2. **Parse**: this gem loads `csharp/csharp<version>.grammar` and drives the `GrammarDrivenParser` to produce a nested `ASTNode` tree.

Both steps are version-aware: the same `version:` string is forwarded to the lexer, ensuring the token grammar and parser grammar always match.

## Usage

```ruby
require "coding_adventures_csharp_parser"

# Default version (C# 12.0)
ast = CodingAdventures::CSharpParser.parse("class Foo { }")
puts ast.rule_name  # => "program"

# Specific version
ast = CodingAdventures::CSharpParser.parse("int x = 1;", version: "8.0")

# Public alias
ast = CodingAdventures::CSharpParser.parse_csharp("int x = 1;")

# Factory method for pipeline workflows
ctx = CodingAdventures::CSharpParser.create_csharp_parser("int x = 1;", version: "12.0")
# ctx => { source: "int x = 1;", version: "12.0", language: :csharp }
```

## Supported Versions

- `"1.0"` -- C# 1.0 (.NET Framework 1.0, 2002)
- `"2.0"` -- C# 2.0 (generics, nullable types, iterators)
- `"3.0"` -- C# 3.0 (LINQ, lambda expressions, `var`)
- `"4.0"` -- C# 4.0 (dynamic binding, named/optional parameters)
- `"5.0"` -- C# 5.0 (`async`/`await`)
- `"6.0"` -- C# 6.0 (expression-bodied members, string interpolation)
- `"7.0"` -- C# 7.0 (pattern matching, tuples, local functions)
- `"8.0"` -- C# 8.0 (nullable reference types, async streams)
- `"9.0"` -- C# 9.0 (records, init-only setters, top-level statements)
- `"10.0"` -- C# 10.0 (global using, file-scoped namespaces)
- `"11.0"` -- C# 11.0 (required members, raw string literals)
- `"12.0"` -- C# 12.0 (primary constructors, collection expressions) *(default)*
- `nil` (default) -- uses C# 12.0 grammar

## Dependencies

- `coding_adventures_grammar_tools` -- reads both `.tokens` and `.grammar` files
- `coding_adventures_lexer` -- the grammar-driven lexer engine
- `coding_adventures_parser` -- the grammar-driven parser engine
- `coding_adventures_csharp_lexer` -- C# tokenization step

## Development

```bash
bundle install
bundle exec rake test
```
