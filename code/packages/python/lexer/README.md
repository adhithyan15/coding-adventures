# Lexer

**Layer 6 of the computing stack** — breaks source code into tokens.

## What this package does

Implements a lexer (tokenizer/scanner) that takes raw source code as input and produces a stream of tokens. This is the first phase of any language processing pipeline: converting a flat string of characters into meaningful lexical units such as keywords, identifiers, literals, and operators.

This is a standalone text-processing package with no dependencies on other packages in the stack.

## Where it fits

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → [Lexer] → Parser → Compiler → VM
```

This package is used by the **parser** package to provide the token stream that gets parsed into an abstract syntax tree.

## Future

Grammar-driven lexer generator — define token rules declaratively and generate a lexer automatically.

## Installation

```bash
uv add coding-adventures-lexer
```

## Usage

```python
from lexer import tokenize

tokens = tokenize("x = 42 + y")
# [Token(IDENTIFIER, "x"), Token(EQUALS, "="), Token(NUMBER, "42"), Token(PLUS, "+"), Token(IDENTIFIER, "y")]
```

## Spec

See [06-lexer.md](../../../specs/06-lexer.md) for the full specification.
