# Lisp Lexer

Tokenizes Lisp source code using the grammar-driven lexer infrastructure.

## What is this?

A thin wrapper around the generic `GrammarLexer` that loads the `lisp.tokens` grammar file. It handles tokenization of McCarthy's 1960 Lisp: numbers, symbols (including operator characters like `+`, `-`, `*`), strings, parentheses, quotes, dots, and comments.

## How it fits in the stack

```
Logic Gates → Arithmetic → CPU → Assembler → [Lexer] → Parser → Compiler → GC → VM
                                                  ↑
                                           lisp.tokens grammar
```

## Usage

```python
from lisp_lexer import tokenize_lisp

tokens = tokenize_lisp("(define square (lambda (x) (* x x)))")
# [Token(LPAREN, '('), Token(SYMBOL, 'define'), Token(SYMBOL, 'square'),
#  Token(LPAREN, '('), Token(SYMBOL, 'lambda'), Token(LPAREN, '('),
#  Token(SYMBOL, 'x'), Token(RPAREN, ')'), Token(LPAREN, '('),
#  Token(SYMBOL, '*'), Token(SYMBOL, 'x'), Token(SYMBOL, 'x'),
#  Token(RPAREN, ')'), Token(RPAREN, ')'), Token(RPAREN, ')'), Token(EOF, '')]
```

## Token types

- `NUMBER` — integers, including negatives: `42`, `-7`
- `SYMBOL` — identifiers and operators: `define`, `+`, `factorial`
- `STRING` — double-quoted strings: `"hello"`
- `LPAREN` / `RPAREN` — `(` and `)`
- `QUOTE` — `'` (syntactic sugar for `(quote ...)`)
- `DOT` — `.` (for dotted pairs: `(a . b)`)
- `EOF` — end of input
