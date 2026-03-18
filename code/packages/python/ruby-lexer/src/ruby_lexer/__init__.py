"""Ruby Lexer — tokenizes Ruby source code using the grammar-driven approach.

This package demonstrates the power of the grammar-driven lexer: by simply
providing a different ``.tokens`` file, the same lexer engine that tokenizes
Python can tokenize Ruby. No new lexer code needed — just a new grammar.

How It Works
------------

The Ruby lexer is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It does three things:

1. Locates the ``ruby.tokens`` file in the ``grammars/`` directory.
2. Parses that file into a ``TokenGrammar`` using ``grammar_tools``.
3. Feeds the grammar to ``GrammarLexer``, which handles the actual
   tokenization.

This is the same pattern used by compiler toolchains like ANTLR: define
the grammar in a data file, and let the engine do the heavy lifting.

Usage::

    from ruby_lexer import tokenize_ruby

    tokens = tokenize_ruby('x = 1 + 2')
    for token in tokens:
        print(token)
"""

from ruby_lexer.tokenizer import create_ruby_lexer, tokenize_ruby

__all__ = [
    "create_ruby_lexer",
    "tokenize_ruby",
]
