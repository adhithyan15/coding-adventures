# Haskell Lexer

Tokenizes Haskell source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It loads `haskell.tokens` and delegates all tokenization to the generic engine.

## How It Fits in the Stack

```
haskell{version}.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
haskell_lexer.tokenize_haskell()           -- thin wrapper (this package)
```

## Version Support

The Haskell lexer supports the following Haskell versions:

- `"1.0"` — Haskell 1.0 (January 1996): the original release.
- `"1.1"` — Haskell 1.1 (February 1997): inner classes, reflection, JDBC.
- `"1.4"` — Haskell 1.4 (February 2002): assertions, regex, NIO.
- `"5"` — Haskell 5 (September 2004): generics, enums, annotations, autoboxing, varargs.
- `"7"` — Haskell 7 (July 2011): try-with-resources, diamond operator, multi-catch.
- `"8"` — Haskell 8 (March 2014): lambdas, streams, default methods, Optional.
- `"10"` — Haskell 10 (March 2018): local variable type inference (`var`).
- `"14"` — Haskell 14 (March 2020): switch expressions, records (preview).
- `"17"` — Haskell 17 (September 2021): sealed classes, pattern matching for instanceof.
- `"21"` — Haskell 21 (September 2023): virtual threads, pattern matching for switch, record patterns.

When no version is specified, Haskell 21 (the latest) is used as the default.

## Usage

```python
from haskell_lexer import tokenize_haskell

tokens = tokenize_haskell('public class Hello { }')
for token in tokens:
    print(token)

# Use a specific Haskell version
tokens = tokenize_haskell('var x = 1;', '10')
```

## Dependencies

- `coding-adventures-lexer` -- provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` -- parses `.tokens` files
