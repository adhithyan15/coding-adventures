# C# Lexer

Tokenizes C# source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It loads `csharp{version}.tokens` and delegates all tokenization to the generic engine.

No new lexer code was written. The same engine that tokenizes Python, JavaScript, and Java tokenizes C# — just with a different grammar file.

## How It Fits in the Stack

```
csharp{version}.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
csharp_lexer.tokenize_csharp()       -- thin wrapper (this package)
```

## Version Support

The C# lexer supports all twelve C# versions:

- `"1.0"` — C# 1.0 (2002): the original release. Classes, interfaces, delegates, properties.
- `"2.0"` — C# 2.0 (2005): generics, nullable types (`int?`), iterators (`yield`).
- `"3.0"` — C# 3.0 (2007): LINQ, lambda expressions (`=>`), `var`, extension methods.
- `"4.0"` — C# 4.0 (2010): `dynamic`, named and optional parameters.
- `"5.0"` — C# 5.0 (2012): async/await (`async`, `await`).
- `"6.0"` — C# 6.0 (2015): null-conditional `?.`, string interpolation `$"..."`, `nameof`.
- `"7.0"` — C# 7.0 (2017): tuples, out variables, pattern matching, local functions.
- `"8.0"` — C# 8.0 (2019): nullable reference types, switch expressions, ranges `..`.
- `"9.0"` — C# 9.0 (2020): records, `init`-only setters, top-level statements.
- `"10.0"` — C# 10.0 (2021): record structs, global using, file-scoped namespaces.
- `"11.0"` — C# 11.0 (2022): `required` members, raw string literals, `file`-local types.
- `"12.0"` — C# 12.0 (2023): primary constructors, collection expressions `[1, 2, 3]`.

When no version is specified, C# 12.0 (the latest) is used as the default.

## Usage

```python
from csharp_lexer import tokenize_csharp

# Default: C# 12.0
tokens = tokenize_csharp('public class Hello { }')
for token in tokens:
    print(token)

# C# 9.0 — records
tokens = tokenize_csharp('record Point(int X, int Y);', '9.0')

# C# 5.0 — async/await
tokens = tokenize_csharp('async Task Run() { await Task.Delay(100); }', '5.0')
```

## Dependencies

- `coding-adventures-lexer` — provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` — parses `.tokens` files
