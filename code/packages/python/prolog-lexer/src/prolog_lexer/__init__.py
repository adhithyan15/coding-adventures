"""Prolog lexer — grammar-driven tokenization for Prolog source code.

This package mirrors the structure of the other language frontends in the repo:
it loads a ``.tokens`` grammar file and delegates the actual tokenization to the
shared :class:`lexer.GrammarLexer`.
"""

from prolog_lexer.tokenizer import create_prolog_lexer, tokenize_prolog

__all__ = ["__version__", "create_prolog_lexer", "tokenize_prolog"]

__version__ = "0.1.0"
