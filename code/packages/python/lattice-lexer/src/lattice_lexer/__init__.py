"""lattice-lexer — Lattice tokenizer for the CSS superset language.

This package tokenizes Lattice source text into a stream of ``Token`` objects.
It is a thin wrapper around the generic ``GrammarLexer``, loading the
``lattice.tokens`` grammar file which defines all CSS tokens plus 5 new
Lattice tokens: ``VARIABLE``, ``EQUALS_EQUALS``, ``NOT_EQUALS``,
``GREATER_EQUALS``, and ``LESS_EQUALS``.

This package is part of the coding-adventures monorepo.
"""

__version__ = "0.1.0"

from lattice_lexer.tokenizer import create_lattice_lexer, tokenize_lattice

__all__ = ["create_lattice_lexer", "tokenize_lattice"]
