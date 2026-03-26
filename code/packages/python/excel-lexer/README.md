# JavaScript Lexer

Tokenizes JavaScript source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It loads `excel.tokens` and delegates all tokenization to the generic engine.

## How It Fits in the Stack

```
excel.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
excel_lexer.tokenize_excel() -- thin wrapper (this package)
```

## Usage

```python
from excel_lexer import tokenize_excel

tokens = tokenize_excel('let x = 1 + 2;')
for token in tokens:
    print(token)
```

## Dependencies

- `coding-adventures-lexer` -- provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` -- parses `.tokens` files
