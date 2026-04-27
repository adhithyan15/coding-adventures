"""MACSYMA Lexer — thin wrapper around the grammar-driven lexer.

This module reads ``macsyma.tokens`` from the repo's ``code/grammars/``
directory and hands it to the generic ``GrammarLexer``. Adding a new
CAS dialect (Mathematica, Maple, etc.) means writing a new ``.tokens``
file — not a line of lexer code.

File location
-------------

The grammar file lives at::

    code/grammars/macsyma/macsyma.tokens

relative to the repository root. This module finds it by walking up
from its own path::

    tokenizer.py
    └── macsyma_lexer/   (parent)
        └── src/         (parent)
            └── macsyma-lexer/  (parent)
                └── python/     (parent)
                    └── packages/ (parent)
                        └── code/ (parent)
                            └── grammars/
                                └── macsyma/
                                    └── macsyma.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# Navigate up from src/macsyma_lexer/tokenizer.py to reach code/grammars/.
# Five .parent calls get us from the file to code/packages/python/; one
# more parent reaches code/packages/; one more reaches code/.
GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
MACSYMA_TOKENS_PATH = GRAMMAR_DIR / "macsyma" / "macsyma.tokens"


def create_macsyma_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for MACSYMA syntax.

    Reads ``macsyma.tokens``, parses it into a ``TokenGrammar``, and
    constructs a ``GrammarLexer`` ready to tokenize the given source.

    Args:
        source: The MACSYMA source text to tokenize.

    Returns:
        A ``GrammarLexer`` instance. Call ``.tokenize()`` to get tokens.

    Raises:
        FileNotFoundError: If the ``macsyma.tokens`` file cannot be found.
    """
    grammar = parse_token_grammar(MACSYMA_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


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
