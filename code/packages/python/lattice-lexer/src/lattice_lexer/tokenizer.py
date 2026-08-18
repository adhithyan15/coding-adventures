"""Lattice Tokenizer — tokenizes Lattice source into token streams.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``lattice.tokens`` file from the ``code/grammars/`` directory and
produces a list of ``Token`` objects.

Lattice extends CSS with 5 new token types:

- ``VARIABLE`` — ``$color``, ``$font-size`` (CSS never uses ``$``)
- ``EQUALS_EQUALS`` — ``==`` (equality comparison in ``@if`` expressions)
- ``NOT_EQUALS`` — ``!=`` (inequality comparison)
- ``GREATER_EQUALS`` — ``>=`` (greater-or-equal comparison)
- ``LESS_EQUALS`` — ``<=`` (less-or-equal comparison)

All CSS token types are preserved unchanged. The ``LINE_COMMENT`` skip
pattern adds support for ``//`` single-line comments (not in CSS).

Pre-compiled Grammar
---------------------

The ``lattice.tokens`` file is compiled ahead of time by the
``grammar-tools`` compiler into ``lattice_lexer/_grammar.py``, which
embeds the ``TokenGrammar`` as native Python data structures. This module
imports ``TOKEN_GRAMMAR`` from it directly — no file I/O or grammar
parsing happens at runtime, and the package works correctly when
installed as a site-package (it does not depend on the
``code/grammars/`` directory existing on disk).

To regenerate after editing ``code/grammars/lattice/lattice.tokens``:

    grammar-tools compile-tokens code/grammars/lattice/lattice.tokens \\
        -o code/packages/python/lattice-lexer/src/lattice_lexer/_grammar.py
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from lattice_lexer._grammar import TOKEN_GRAMMAR


def create_lattice_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Lattice source text.

    This function uses the pre-compiled ``TOKEN_GRAMMAR`` (from
    ``lattice_lexer._grammar``) to create a ``GrammarLexer`` with the
    Lattice token definitions. No file I/O is performed.

    Args:
        source: The Lattice source text to tokenize.

    Returns:
        A ``GrammarLexer`` instance ready to produce tokens.

    Example::

        lexer = create_lattice_lexer('$color: red;')
        tokens = lexer.tokenize()
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_lattice(source: str) -> list[Token]:
    """Tokenize Lattice source text and return a list of tokens.

    This is the main entry point for the Lattice tokenizer. Pass in a
    string of Lattice source, get back a list of ``Token`` objects.

    The returned list always ends with an ``EOF`` token.

    Args:
        source: The Lattice source text to tokenize.

    Returns:
        A list of ``Token`` objects.

    Raises:
        LexerError: If the source contains characters that don't match
            any token pattern.

    Example::

        tokens = tokenize_lattice('$color: red;')
        # [Token(VARIABLE, '$color'), Token(COLON, ':'),
        #  Token(IDENT, 'red'), Token(SEMICOLON, ';'), Token(EOF, '')]
    """
    lexer = create_lattice_lexer(source)
    return lexer.tokenize()
