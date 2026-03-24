# SQL Lexer

Tokenizes ANSI SQL text using the grammar-driven lexer — a thin wrapper
that loads `sql.tokens` and feeds it to the generic `GrammarLexer`.

## What Is This?

This package is a **thin wrapper** around the grammar-driven `GrammarLexer`. It
demonstrates the grammar-driven architecture applied to a real, widely-used query
language. The same lexer engine that tokenizes Python, Ruby, and JSON can tokenize
SQL — just by swapping the `.tokens` file.

SQL is more complex than JSON: it has case-insensitive keywords, comparison and
arithmetic operators, single-quoted string literals, backtick-quoted identifiers,
and two comment styles. All of this is handled declaratively in `sql.tokens`.

## How It Fits in the Stack

```
sql.tokens  (declarative token definitions, @case_insensitive true)
    |
grammar_tools.parse_token_grammar()
    |
GrammarLexer  (generic engine from lexer package)
    |
sql_lexer.tokenize_sql()  (this thin wrapper)
    |
    v
list[Token]  (KEYWORD, NAME, NUMBER, STRING, operators, punctuation)
```

## Usage

```python
from sql_lexer import tokenize_sql

tokens = tokenize_sql("SELECT id FROM users WHERE active = TRUE")
for token in tokens:
    print(f"{token.type}: {token.value!r}")
```

Output:
```
KEYWORD: 'SELECT'
NAME: 'id'
KEYWORD: 'FROM'
NAME: 'users'
KEYWORD: 'WHERE'
NAME: 'active'
EQUALS: '='
KEYWORD: 'TRUE'
EOF: ''
```

## Case Insensitivity

SQL keywords are case-insensitive. The `sql.tokens` grammar includes
`# @case_insensitive true`, which causes all keyword values to be normalized
to uppercase:

```python
tokenize_sql("select")   # → KEYWORD("SELECT")
tokenize_sql("SELECT")   # → KEYWORD("SELECT")
tokenize_sql("Select")   # → KEYWORD("SELECT")
```

## Installation

```bash
pip install coding-adventures-sql-lexer
```

## Dependencies

- `coding-adventures-grammar-tools` — parses the `.tokens` file
- `coding-adventures-lexer` — provides the `GrammarLexer` engine
