"""C# Lexer — tokenizes C# source code using the grammar-driven approach.

This package demonstrates the power of the grammar-driven lexer: by simply
providing a different ``.tokens`` file, the same lexer engine that tokenizes
Python, JavaScript, or Java can tokenize C#. No new lexer code needed — just
a new grammar.

How It Works
------------

The C# lexer is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It does three things:

1. Locates the ``csharp{version}.tokens`` file in the ``grammars/csharp/``
   directory.
2. Parses that file into a ``TokenGrammar`` using ``grammar_tools``.
3. Feeds the grammar to ``GrammarLexer``, which handles the actual
   tokenization.

The lexer supports all twelve C# versions from 1.0 (2002) through 12.0
(2023), letting you reproduce the exact token stream that a specific
version of the C# compiler would see.

Usage::

    from csharp_lexer import tokenize_csharp

    tokens = tokenize_csharp('public class Hello { }')
    for token in tokens:
        print(token)

    # Using a specific version
    tokens = tokenize_csharp('record Point(int X, int Y);', '9.0')
"""

from csharp_lexer.tokenizer import create_csharp_lexer, tokenize_csharp

__all__ = [
    "create_csharp_lexer",
    "tokenize_csharp",
]
