"""SQL Lexer — tokenizes SQL text using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarLexer``. It
demonstrates the grammar-driven architecture applied to a real, widely-used
query language: ANSI SQL.

Unlike JSON (which has no keywords, operators, or comments), SQL is a rich
language with case-insensitive keywords, multiple operator spellings (both
``!=`` and ``<>`` for not-equals), single-quoted string literals, backtick-quoted
identifiers, and two comment styles (``--`` line comments and ``/* */`` block
comments).

The ``sql.tokens`` grammar handles all of this declaratively. The
``# @case_insensitive true`` directive causes the lexer to normalize keyword
values to uppercase, so ``select``, ``SELECT``, and ``Select`` all produce
``KEYWORD("SELECT")``.

Usage::

    from sql_lexer import tokenize_sql

    tokens = tokenize_sql("SELECT id FROM users WHERE active = TRUE")
    for token in tokens:
        print(token)
"""

from sql_lexer.tokenizer import create_sql_lexer, tokenize_sql

__all__ = [
    "create_sql_lexer",
    "tokenize_sql",
]
