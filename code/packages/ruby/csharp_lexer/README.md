# C# Lexer

A Ruby gem that tokenizes C# source code using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding C#-specific tokenization rules, it loads the `csharp/csharp<version>.tokens` grammar file and feeds it to the general-purpose lexer engine.

C# is a statically typed, object-oriented language from Microsoft, part of the .NET platform. It shares many keywords with Java but also introduces C#-specific operators like `??` (null-coalescing), `?.` (null-conditional), and `=>` (lambda arrow / expression body).

## Usage

```ruby
require "coding_adventures_csharp_lexer"

# Default version (C# 12.0)
tokens = CodingAdventures::CSharpLexer.tokenize("class Foo { }")
tokens.each { |t| puts t }

# Specific version
tokens = CodingAdventures::CSharpLexer.tokenize("int x = 1;", version: "8.0")

# Public alias
tokens = CodingAdventures::CSharpLexer.tokenize_csharp("int x = 1;")

# Factory method for pipeline workflows
lexer = CodingAdventures::CSharpLexer.create_csharp_lexer("class Foo { }", version: "12.0")
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

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
