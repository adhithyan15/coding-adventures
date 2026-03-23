# TOML Lexer

Tokenizes TOML text (v1.0.0) using the grammar-driven lexer — a thin wrapper
that loads `toml.tokens` and feeds it to the generic `GrammarLexer`.

## What Is This?

This package is a **thin wrapper** around the grammar-driven `GrammarLexer`. It
demonstrates that the same lexer engine used for Python, Ruby, JavaScript, and
JSON can tokenize TOML — just by swapping the `.tokens` file. No new lexer code
is needed.

TOML is more complex than JSON but simpler than a full programming language. It
exercises several lexer features that JSON does not: newline sensitivity, four
string types, date/time literals, comments, and bare keys. This makes it an
excellent second validation case for the grammar-driven architecture.

## How It Fits in the Stack

```
toml.tokens  (declarative token definitions)
    |
grammar_tools.parse_token_grammar()
    |
GrammarLexer  (generic engine from lexer package)
    |
toml_lexer.tokenize_toml()  (this thin wrapper)
    |
    v
list[Token]  (20 token types: strings, numbers, dates, keys, delimiters)
```

## Token Types

| Category    | Token Types                                                           |
|-------------|-----------------------------------------------------------------------|
| Strings     | BASIC_STRING, ML_BASIC_STRING, LITERAL_STRING, ML_LITERAL_STRING      |
| Numbers     | INTEGER, FLOAT                                                        |
| Booleans    | TRUE, FALSE                                                           |
| Date/Times  | OFFSET_DATETIME, LOCAL_DATETIME, LOCAL_DATE, LOCAL_TIME               |
| Keys        | BARE_KEY                                                              |
| Delimiters  | EQUALS, DOT, COMMA, LBRACKET, RBRACKET, LBRACE, RBRACE              |
| Structural  | NEWLINE, EOF                                                          |

## Usage

```python
from toml_lexer import tokenize_toml

tokens = tokenize_toml('[server]\nhost = "localhost"\nport = 8080')
for token in tokens:
    print(f"{token.type}: {token.value}")
```

## Installation

```bash
pip install coding-adventures-toml-lexer
```

## Dependencies

- `coding-adventures-grammar-tools` — parses the `.tokens` file
- `coding-adventures-lexer` — provides the `GrammarLexer` engine
