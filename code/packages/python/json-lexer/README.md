# JSON Lexer

Tokenizes JSON text (RFC 8259) using the grammar-driven lexer — a thin wrapper
that loads `json.tokens` and feeds it to the generic `GrammarLexer`.

## What Is This?

This package is a **thin wrapper** around the grammar-driven `GrammarLexer`. It
demonstrates a core idea of the grammar-driven architecture: the same lexer engine
that tokenizes Python, Ruby, JavaScript, and Starlark can tokenize JSON — just by
swapping the `.tokens` file. No new lexer code is needed.

JSON is the simplest grammar the infrastructure supports: no keywords, no operators,
no comments, no indentation. It serves as the ideal validation case for the
grammar-driven approach.

## How It Fits in the Stack

```
json.tokens  (declarative token definitions)
    |
grammar_tools.parse_token_grammar()
    |
GrammarLexer  (generic engine from lexer package)
    |
json_lexer.tokenize_json()  (this thin wrapper)
    |
    v
list[Token]  (STRING, NUMBER, TRUE, FALSE, NULL, {, }, [, ], :, ,)
```

## Usage

```python
from json_lexer import tokenize_json

tokens = tokenize_json('{"name": "Ada", "age": 36}')
for token in tokens:
    print(f"{token.type}: {token.value}")
```

## Installation

```bash
pip install coding-adventures-json-lexer
```

## Dependencies

- `coding-adventures-grammar-tools` — parses the `.tokens` file
- `coding-adventures-lexer` — provides the `GrammarLexer` engine
