# Starlark Lexer

Tokenizes Starlark source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It demonstrates a core principle of the grammar-driven architecture: the same lexer engine that tokenizes Python can tokenize Starlark by simply loading a different `.tokens` file.

No new lexer code is needed. The `starlark.tokens` file in `code/grammars/` declares Starlark's token definitions (keywords, operators, literals, indentation mode), and the `GrammarLexer` reads those declarations at runtime.

## What Is Starlark?

Starlark is a deterministic subset of Python designed by Google for BUILD files (Bazel, Buck). It removes features that make evaluation unpredictable (no `while` loops, no recursion, no `class`, no `import`), while keeping Python's familiar syntax for `def`, `for`, `if/elif/else`, list comprehensions, and dictionaries.

## How It Fits in the Stack

```
starlark.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
starlark_lexer.tokenize_starlark()   -- thin wrapper (this package)
```

## Key Features

- **Indentation mode**: Automatically generates `INDENT`, `DEDENT`, and `NEWLINE` tokens based on whitespace, just like Python.
- **Bracket suppression**: Inside `()`, `[]`, `{}`, indentation tokens are suppressed for multi-line expressions.
- **Reserved keywords**: Words like `class`, `import`, `while` cause immediate lex errors to prevent Python code from being silently misinterpreted.
- **Type aliases**: All string quoting styles (`"..."`, `'...'`, `"""..."""`, `'''...'''`, plus `r`/`b` prefixes) emit a single `STRING` token type.

## Usage

```python
from starlark_lexer import tokenize_starlark

# Tokenize a simple Starlark expression
tokens = tokenize_starlark('x = 1 + 2')
for token in tokens:
    print(token)
# Token(NAME, 'x', 1:1)
# Token(EQUALS, '=', 1:3)
# Token(INT, '1', 1:5)
# Token(PLUS, '+', 1:7)
# Token(INT, '2', 1:9)
# Token(NEWLINE, ...)
# Token(EOF, '', ...)

# Starlark keywords are recognized
tokens = tokenize_starlark('def greet(name):')
# Token(KEYWORD, 'def', 1:1)
# Token(NAME, 'greet', 1:5)
# Token(LPAREN, '(', 1:10)
# Token(NAME, 'name', 1:11)
# Token(RPAREN, ')', 1:15)
# Token(COLON, ':', 1:16)
# ...

# Indented blocks produce INDENT/DEDENT tokens
tokens = tokenize_starlark('def f():\n    return 1\n')
# Token(KEYWORD, 'def', ...)
# Token(NAME, 'f', ...)
# Token(LPAREN, ...)
# Token(RPAREN, ...)
# Token(COLON, ...)
# Token(NEWLINE, ...)
# Token(INDENT, ...)
# Token(KEYWORD, 'return', ...)
# Token(INT, '1', ...)
# Token(NEWLINE, ...)
# Token(DEDENT, ...)
# Token(EOF, ...)
```

## Installation

```bash
pip install coding-adventures-starlark-lexer
```

## Dependencies

- `coding-adventures-lexer` -- provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` -- parses `.tokens` files
