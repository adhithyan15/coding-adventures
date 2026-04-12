"""Java Lexer — tokenizes Java source code using the grammar-driven approach.

This package demonstrates the power of the grammar-driven lexer: by simply
providing a different ``.tokens`` file, the same lexer engine that tokenizes
Python or JavaScript can tokenize Java. No new lexer code needed — just a
new grammar.

How It Works
------------

The Java lexer is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It does three things:

1. Locates the ``java{version}.tokens`` file in the ``grammars/java/`` directory.
2. Parses that file into a ``TokenGrammar`` using ``grammar_tools``.
3. Feeds the grammar to ``GrammarLexer``, which handles the actual
   tokenization.

Usage::

    from java_lexer import tokenize_java

    tokens = tokenize_java('public class Hello { }')
    for token in tokens:
        print(token)
"""

from java_lexer.tokenizer import create_java_lexer, tokenize_java

__all__ = [
    "create_java_lexer",
    "tokenize_java",
]
