"""Haskell Lexer â€” tokenizes Haskell source code using the grammar-driven approach.

This package demonstrates the power of the grammar-driven lexer: by simply
providing a different ``.tokens`` file, the same lexer engine that tokenizes
Python or HaskellScript can tokenize Haskell. No new lexer code needed â€” just a
new grammar.

How It Works
------------

The Haskell lexer is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It does three things:

1. Locates the ``haskell{version}.tokens`` file in the ``grammars/haskell/`` directory.
2. Parses that file into a ``TokenGrammar`` using ``grammar_tools``.
3. Feeds the grammar to ``GrammarLexer``, which handles the actual
   tokenization.

Usage::

    from haskell_lexer import tokenize_haskell

    tokens = tokenize_haskell('public class Hello { }')
    for token in tokens:
        print(token)
"""

from haskell_lexer.lexer import create_haskell_lexer, tokenize_haskell

__all__ = [
    "create_haskell_lexer",
    "tokenize_haskell",
]

