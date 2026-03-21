"""JavaScript Lexer — tokenizes JavaScript source code using the grammar-driven approach.

This package demonstrates the power of the grammar-driven lexer: by simply
providing a different ``.tokens`` file, the same lexer engine that tokenizes
Python can tokenize JavaScript. No new lexer code needed — just a new grammar.

How It Works
------------

The JavaScript lexer is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It does three things:

1. Locates the ``javascript.tokens`` file in the ``grammars/`` directory.
2. Parses that file into a ``TokenGrammar`` using ``grammar_tools``.
3. Feeds the grammar to ``GrammarLexer``, which handles the actual
   tokenization.

Usage::

    from javascript_lexer import tokenize_javascript

    tokens = tokenize_javascript('let x = 1 + 2;')
    for token in tokens:
        print(token)
"""

from javascript_lexer.tokenizer import create_javascript_lexer, tokenize_javascript

__all__ = [
    "create_javascript_lexer",
    "tokenize_javascript",
]
