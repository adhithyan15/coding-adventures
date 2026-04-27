# Java Lexer

Tokenizes Java source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It loads `java.tokens` and delegates all tokenization to the generic engine.

## How It Fits in the Stack

```
java{version}.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
java_lexer.tokenize_java()           -- thin wrapper (this package)
```

## Version Support

The Java lexer supports the following Java versions:

- `"1.0"` — Java 1.0 (January 1996): the original release.
- `"1.1"` — Java 1.1 (February 1997): inner classes, reflection, JDBC.
- `"1.4"` — Java 1.4 (February 2002): assertions, regex, NIO.
- `"5"` — Java 5 (September 2004): generics, enums, annotations, autoboxing, varargs.
- `"7"` — Java 7 (July 2011): try-with-resources, diamond operator, multi-catch.
- `"8"` — Java 8 (March 2014): lambdas, streams, default methods, Optional.
- `"10"` — Java 10 (March 2018): local variable type inference (`var`).
- `"14"` — Java 14 (March 2020): switch expressions, records (preview).
- `"17"` — Java 17 (September 2021): sealed classes, pattern matching for instanceof.
- `"21"` — Java 21 (September 2023): virtual threads, pattern matching for switch, record patterns.

When no version is specified, Java 21 (the latest) is used as the default.

## Usage

```python
from java_lexer import tokenize_java

tokens = tokenize_java('public class Hello { }')
for token in tokens:
    print(token)

# Use a specific Java version
tokens = tokenize_java('var x = 1;', '10')
```

## Dependencies

- `coding-adventures-lexer` -- provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` -- parses `.tokens` files
