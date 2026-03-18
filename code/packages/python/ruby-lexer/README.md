# Ruby Lexer

Tokenizes Ruby source code using the grammar-driven lexer approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarLexer` from the `lexer` package. It demonstrates a core principle of the grammar-driven architecture: the same lexer engine that tokenizes Python can tokenize Ruby by simply loading a different `.tokens` file.

No new lexer code is needed. The `ruby.tokens` file in `code/grammars/` declares Ruby's token definitions (keywords, operators, literals), and the `GrammarLexer` reads those declarations at runtime.

## How It Fits in the Stack

```
ruby.tokens (grammar file)
    |
    v
grammar_tools.parse_token_grammar()  -- parses the .tokens file
    |
    v
lexer.GrammarLexer                   -- generic tokenization engine
    |
    v
ruby_lexer.tokenize_ruby()           -- thin wrapper (this package)
```

## Usage

```python
from ruby_lexer import tokenize_ruby

# Tokenize a simple Ruby expression
tokens = tokenize_ruby('x = 1 + 2')
for token in tokens:
    print(token)
# Token(NAME, 'x', 1:1)
# Token(EQUALS, '=', 1:3)
# Token(NUMBER, '1', 1:5)
# Token(PLUS, '+', 1:7)
# Token(NUMBER, '2', 1:9)
# Token(EOF, '', 1:10)

# Ruby keywords are recognized
tokens = tokenize_ruby('def greet(name)')
# Token(KEYWORD, 'def', 1:1)
# Token(NAME, 'greet', 1:5)
# Token(LPAREN, '(', 1:10)
# Token(NAME, 'name', 1:11)
# Token(RPAREN, ')', 1:15)
# Token(EOF, '', 1:16)
```

## Installation

```bash
pip install coding-adventures-ruby-lexer
```

## Dependencies

- `coding-adventures-lexer` -- provides `GrammarLexer` and `Token`
- `coding-adventures-grammar-tools` -- parses `.tokens` files
