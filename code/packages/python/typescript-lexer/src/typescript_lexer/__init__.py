"""TypeScript Lexer — tokenizes TypeScript source code using the grammar-driven approach.

This package demonstrates the power of the grammar-driven lexer: by simply
providing a different ``.tokens`` file, the same lexer engine that tokenizes
Python and JavaScript can tokenize TypeScript. No new lexer code needed — just
a new grammar.

How It Works
------------

The TypeScript lexer is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It does three things:

1. Locates the ``typescript.tokens`` file in the ``grammars/`` directory.
2. Parses that file into a ``TokenGrammar`` using ``grammar_tools``.
3. Feeds the grammar to ``GrammarLexer``, which handles the actual
   tokenization.

Usage::

    from typescript_lexer import tokenize_typescript

    tokens = tokenize_typescript('let x: number = 1 + 2;')
    for token in tokens:
        print(token)
"""

from typescript_lexer.tokenizer import create_typescript_lexer, tokenize_typescript

__all__ = [
    "create_typescript_lexer",
    "tokenize_typescript",
]
