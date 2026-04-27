"""MACSYMA Lexer — tokenizes MACSYMA/Maxima expression syntax.

This package is a thin wrapper around the generic ``GrammarLexer``. It
loads ``macsyma.tokens`` from the ``code/grammars/macsyma/`` directory
and produces a stream of ``Token`` objects ready for the parser.

MACSYMA is the grandparent of every modern computer-algebra system —
its lexical conventions (``:`` for assignment, ``:=`` for function
definition, ``;`` vs ``$`` terminators, ``#`` for not-equal, ``/* */``
comments) are shared by Maxima and influenced Mathematica, Maple, and
REDUCE.

Usage::

    from macsyma_lexer import tokenize_macsyma

    tokens = tokenize_macsyma("f(x) := x^2; diff(f(x), x);")
"""

from macsyma_lexer.tokenizer import create_macsyma_lexer, tokenize_macsyma

__all__ = [
    "create_macsyma_lexer",
    "tokenize_macsyma",
]
