# Python Lexer

Tokenizes Python source code using versioned grammar files, supporting Python 2.7 through 3.12.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It demonstrates versioned grammar support: different `.tokens` files for each Python version capture the exact token set for that version, while the lexer engine remains unchanged.

No new lexer code is needed per version. The `pythonX.Y.tokens` files in `code/grammars/python/` declare each version's token definitions (keywords, operators, literals, indentation mode), and the `GrammarLexer` reads those declarations at runtime.

## Why Versioned Grammars?

Python's lexical grammar has changed significantly over the years:

- **Python 2.7 vs 3.0**: `print` and `exec` changed from keywords to built-in functions
- **Python 3.6**: f-strings (`f"..."`) were introduced
- **Python 3.8**: The walrus operator (`:=`) was added
- **Python 3.10**: `match` and `case` became soft keywords
- **Python 3.12**: `type` became a soft keyword

Each version's grammar file precisely captures the lexical rules for that version.

## How It Fits in the Stack

```
python3.12.tokens (grammar file, version-specific)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
python_lexer.tokenize_python()       -- thin wrapper (this package)
```

## Supported Versions

| Version | Notable Lexical Features |
|---------|------------------------|
| 2.7     | `print`/`exec` keywords, `<>` operator |
| 3.0     | `print`/`exec` become names, `nonlocal` keyword |
| 3.6     | f-string prefix, underscores in numeric literals |
| 3.8     | Walrus operator `:=` |
| 3.10    | `match`/`case` soft keywords |
| 3.12    | `type` soft keyword |

## Key Features

- **Version selection**: Choose which Python version's grammar to use
- **Grammar caching**: Parsed grammars are cached per version for performance
- **Indentation mode**: Automatic `INDENT`, `DEDENT`, and `NEWLINE` token generation
- **Bracket suppression**: Inside `()`, `[]`, `{}`, indentation tokens are suppressed
- **Type aliases**: All string quoting styles emit a single `STRING` token type

## Usage

```python
from python_lexer import tokenize_python

# Default version (3.12)
tokens = tokenize_python('x = 1 + 2')
for token in tokens:
    print(token)
# Token(NAME, 'x', 1:1)
# Token(EQUALS, '=', 1:3)
# Token(INT, '1', 1:5)
# Token(PLUS, '+', 1:7)
# Token(INT, '2', 1:9)
# Token(NEWLINE, ...)
# Token(EOF, '', ...)

# Use a specific version
tokens = tokenize_python('print "hello"', version="2.7")
# Token(KEYWORD, 'print', 1:1)  -- print is a keyword in 2.7
# Token(STRING, 'hello', 1:7)
# ...

# Python 3.0+ -- print is a regular name
tokens = tokenize_python('print("hello")', version="3.0")
# Token(NAME, 'print', 1:1)  -- print is a name in 3.0+
# Token(LPAREN, '(', 1:6)
# Token(STRING, 'hello', 1:7)
# Token(RPAREN, ')', 1:14)
# ...
```

## Installation

```bash
pip install coding-adventures-python-lexer
```

## Dependencies

- `coding-adventures-lexer` -- provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` -- parses `.tokens` files
