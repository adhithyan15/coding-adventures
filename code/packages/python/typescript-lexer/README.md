# TypeScript Lexer

Tokenizes TypeScript source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It loads `typescript.tokens` and delegates all tokenization to the generic engine.

## How It Fits in the Stack

```
typescript.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
typescript_lexer.tokenize_typescript() -- thin wrapper (this package)
```

## Usage

```python
from typescript_lexer import tokenize_typescript

tokens = tokenize_typescript('let x: number = 1 + 2;')
for token in tokens:
    print(token)
```

## Dependencies

- `coding-adventures-lexer` -- provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` -- parses `.tokens` files
