# JavaScript Lexer

Tokenizes JavaScript source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It loads `javascript.tokens` and delegates all tokenization to the generic engine.

## How It Fits in the Stack

```
javascript.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
javascript_lexer.tokenize_javascript() -- thin wrapper (this package)
```

## Usage

```python
from javascript_lexer import tokenize_javascript

tokens = tokenize_javascript('let x = 1 + 2;')
for token in tokens:
    print(token)
```

## Dependencies

- `coding-adventures-lexer` -- provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` -- parses `.tokens` files
