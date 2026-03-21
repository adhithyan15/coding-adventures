"""Starlark Lexer — tokenizes Starlark source code using the grammar-driven approach.

This package demonstrates the power of the grammar-driven lexer: by simply
providing a different ``.tokens`` file, the same lexer engine that tokenizes
Python can tokenize Starlark. No new lexer code needed — just a new grammar.

What Is Starlark?
-----------------

Starlark is a deterministic subset of Python, designed by Google for use in
BUILD files (Bazel, Buck, etc.). It intentionally removes features that make
evaluation unpredictable:

- No ``while`` loops or recursion (guarantees termination)
- No ``class`` definitions (keeps the language simple)
- No ``import`` statement (uses ``load()`` instead)
- No ``try``/``except`` (errors are always fatal)

Despite these restrictions, Starlark looks and feels like Python. It has
``def``, ``for``, ``if/elif/else``, list comprehensions, and dictionaries.

How It Works
------------

The Starlark lexer is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It does three things:

1. Locates the ``starlark.tokens`` file in the ``grammars/`` directory.
2. Parses that file into a ``TokenGrammar`` using ``grammar_tools``.
3. Feeds the grammar to ``GrammarLexer``, which handles the actual
   tokenization.

Key Differences from the Ruby Lexer
------------------------------------

Unlike the Ruby lexer, the Starlark lexer uses **indentation mode**. This
means the lexer automatically generates ``INDENT``, ``DEDENT``, and
``NEWLINE`` tokens based on whitespace changes — just like Python. Inside
brackets (parentheses, square brackets, curly braces), indentation is
suppressed to allow multi-line expressions.

Starlark also has **reserved keywords** (like ``class``, ``import``, ``while``)
that cause a lex error if encountered. This prevents users from writing
Python code that looks valid but would behave differently in Starlark.

Usage::

    from starlark_lexer import tokenize_starlark

    tokens = tokenize_starlark('x = 1 + 2')
    for token in tokens:
        print(token)
"""

from starlark_lexer.tokenizer import create_starlark_lexer, tokenize_starlark

__all__ = [
    "create_starlark_lexer",
    "tokenize_starlark",
]
