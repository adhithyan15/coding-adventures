"""MACSYMA Lexer — thin wrapper around the grammar-driven lexer.

The MACSYMA token grammar (``code/grammars/macsyma/macsyma.tokens``) is
pre-compiled into ``macsyma_lexer._grammar`` — see that module's header
for regeneration instructions. Adding a new CAS dialect (Mathematica,
Maple, etc.) means writing a new ``.tokens`` file and compiling it — not
a line of lexer code.
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from macsyma_lexer._grammar import TOKEN_GRAMMAR


def create_macsyma_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for MACSYMA syntax.

    Uses the pre-compiled ``TOKEN_GRAMMAR`` constant — no file I/O.

    Args:
        source: The MACSYMA source text to tokenize.

    Returns:
        A ``GrammarLexer`` instance. Call ``.tokenize()`` to get tokens.
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_macsyma(source: str) -> list[Token]:
    """Tokenize MACSYMA source text and return a list of tokens.

    This is the main entry point. Pass in a MACSYMA expression or
    program, get back a flat token list ending in ``EOF``.

    Token types produced include:

    - ``NUMBER`` — integer or float literal.
    - ``NAME`` — identifier (including ``%pi``, ``%e``, ``%i``).
    - ``STRING`` — double-quoted string literal.
    - ``KEYWORD`` — reserved word (``and``, ``or``, ``not``, ``true``,
      ``false``, ``if``, ``then``, ``else``, ``for``, ``while``, etc.).
      The ``value`` field holds the actual keyword text.
    - Operator tokens: ``PLUS``, ``MINUS``, ``STAR``, ``SLASH``,
      ``CARET``, ``STAREQ``, ``COLON``, ``COLONEQ``, ``EQ``, ``HASH``,
      ``LT``, ``GT``, ``LEQ``, ``GEQ``, ``ARROW``, ``BANG``.
    - Delimiter tokens: ``LPAREN``, ``RPAREN``, ``LBRACKET``,
      ``RBRACKET``, ``LBRACE``, ``RBRACE``, ``COMMA``, ``SEMI``,
      ``DOLLAR``.
    - ``EOF`` — always the last token.

    Args:
        source: The MACSYMA source text.

    Returns:
        A list of ``Token`` objects. The last is always EOF.

    Example::

        tokens = tokenize_macsyma("x^2 + 1;")
        # NAME('x'), CARET('^'), NUMBER('2'), PLUS('+'),
        # NUMBER('1'), SEMI(';'), EOF
    """
    return create_macsyma_lexer(source).tokenize()
