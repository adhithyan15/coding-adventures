# C# Parser

Parses C# source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It loads `csharp{version}.grammar` and delegates all parsing to the generic engine.

No new parser code was written. The same engine that parses Java, Python, or JavaScript parses C# — just with a different grammar file.

## How It Fits in the Stack

```
C# source code
    |
    v
csharp_lexer.tokenize_csharp()         -- tokenizes using csharp{version}.tokens
    |
    v
csharp{version}.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar()   -- parses the .grammar file
    |
    v
lang_parser.GrammarParser              -- generic parsing engine
    |
    v
csharp_parser.parse_csharp()           -- thin wrapper (this package)
    |
    v
ASTNode tree                           -- generic AST
```

## Version Support

Supports all twelve C# versions:

- `"1.0"` — C# 1.0 (2002): the original release.
- `"2.0"` — C# 2.0 (2005): generics, nullable types (`int?`), iterators.
- `"3.0"` — C# 3.0 (2007): LINQ, lambda expressions, `var`, extension methods.
- `"4.0"` — C# 4.0 (2010): `dynamic`, named and optional parameters.
- `"5.0"` — C# 5.0 (2012): async/await.
- `"6.0"` — C# 6.0 (2015): `?.`, `$"..."`, `nameof`, expression-bodied members.
- `"7.0"` — C# 7.0 (2017): tuples, out variables, pattern matching, local functions.
- `"8.0"` — C# 8.0 (2019): nullable reference types, switch expressions, ranges.
- `"9.0"` — C# 9.0 (2020): records, `init`-only setters, top-level statements.
- `"10.0"` — C# 10.0 (2021): record structs, global using, file-scoped namespaces.
- `"11.0"` — C# 11.0 (2022): `required` members, raw string literals, `file`-local types.
- `"12.0"` — C# 12.0 (2023): primary constructors, collection expressions.

Default is C# 12.0 (latest).

## Usage

```python
from csharp_parser import parse_csharp

ast = parse_csharp('public class Hello { }')
print(ast.rule_name)  # "program"

# Use a specific C# version
ast = parse_csharp('record Point(int X, int Y);', '9.0')

# Namespace
ast = parse_csharp('namespace MyApp { }')
```

## Dependencies

- `coding-adventures-csharp-lexer` — tokenizes C# source code
- `coding-adventures-parser` — provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` — parses `.grammar` files
