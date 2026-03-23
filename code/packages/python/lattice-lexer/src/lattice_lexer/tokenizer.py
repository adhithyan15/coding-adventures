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

Locating the Grammar File
--------------------------

The ``lattice.tokens`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    tokenizer.py
    └── lattice_lexer/       (parent)
        └── src/             (parent)
            └── lattice-lexer/   (parent)
                └── python/      (parent)
                    └── packages/ (parent)
                        └── code/ (parent)
                            └── grammars/
                                └── lattice.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
LATTICE_TOKENS_PATH = GRAMMAR_DIR / "lattice.tokens"


def create_lattice_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Lattice source text.

    This function:

    1. Reads and parses the ``lattice.tokens`` grammar file.
    2. Creates a ``GrammarLexer`` with the Lattice token definitions.

    Args:
        source: The Lattice source text to tokenize.

    Returns:
        A ``GrammarLexer`` instance ready to produce tokens.

    Raises:
        FileNotFoundError: If the grammar file cannot be found.

    Example::

        lexer = create_lattice_lexer('$color: red;')
        tokens = lexer.tokenize()
    """
    token_grammar = parse_token_grammar(LATTICE_TOKENS_PATH.read_text())
    return GrammarLexer(source, token_grammar)


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
        FileNotFoundError: If the grammar file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern.

    Example::

        tokens = tokenize_lattice('$color: red;')
        # [Token(VARIABLE, '$color'), Token(COLON, ':'),
        #  Token(IDENT, 'red'), Token(SEMICOLON, ';'), Token(EOF, '')]
    """
    lexer = create_lattice_lexer(source)
    return lexer.tokenize()
