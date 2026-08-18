"""JSON Lexer — tokenizes JSON text using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``json.tokens`` file from the ``code/grammars/`` directory and creates a
lexer configured for JSON tokenization.

JSON (RFC 8259) is the simplest grammar the infrastructure supports. Unlike
programming languages, JSON has:

- **No keywords.** The values ``true``, ``false``, and ``null`` are defined
  as literal tokens (TRUE, FALSE, NULL) rather than being reclassified from
  a NAME token. JSON has no identifier concept at all.
- **No operators.** There is no ``+``, ``-``, ``=``, or any other operator.
  (The minus sign in numbers like ``-42`` is part of the NUMBER regex, not a
  separate operator token.)
- **No comments.** JSON does not support comments of any kind.
- **No indentation significance.** All whitespace (including newlines) is
  handled by a ``skip:`` pattern that consumes it silently.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_json_lexer(source)`` — creates a ``GrammarLexer`` configured for
  JSON. Use this when you want to control the tokenization process yourself.
- ``tokenize_json(source)`` — the all-in-one function. Pass in JSON text,
  get back a list of tokens. This is the function most callers want.

Pre-compiled Grammar
---------------------

The ``json.tokens`` file is compiled ahead of time by the ``grammar-tools``
compiler into ``json_lexer/_grammar.py``, which embeds the
``TokenGrammar`` as native Python data structures. This module imports
``TOKEN_GRAMMAR`` from it directly — no file I/O or grammar parsing
happens at runtime, and the package works correctly when installed as a
site-package (it does not depend on the ``code/grammars/`` directory
existing on disk).

To regenerate after editing ``code/grammars/json/json.tokens``:

    grammar-tools compile-tokens code/grammars/json/json.tokens \\
        -o code/packages/python/json-lexer/src/json_lexer/_grammar.py
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from json_lexer._grammar import TOKEN_GRAMMAR


def create_json_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for JSON text.

    This function uses the pre-compiled ``TOKEN_GRAMMAR`` (from
    ``json_lexer._grammar``) to create a ``GrammarLexer`` ready to tokenize
    the given source text. No file I/O is performed.

    Args:
        source: The JSON text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with JSON token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Example::

        lexer = create_json_lexer('{"key": "value"}')
        tokens = lexer.tokenize()
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_json(source: str) -> list[Token]:
    """Tokenize JSON text and return a list of tokens.

    This is the main entry point for the JSON lexer. Pass in a string of
    JSON text, and get back a flat list of ``Token`` objects. The list
    always ends with an ``EOF`` token.

    The 11 token types you will see are:

    - **STRING** — a double-quoted string (quotes stripped, escapes processed)
    - **NUMBER** — an integer or floating-point number (may be negative)
    - **TRUE** — the literal ``true``
    - **FALSE** — the literal ``false``
    - **NULL** — the literal ``null``
    - **LBRACE** / **RBRACE** — ``{`` and ``}``
    - **LBRACKET** / **RBRACKET** — ``[`` and ``]``
    - **COLON** — ``:``
    - **COMMA** — ``,``
    - **EOF** — end of input

    Args:
        source: The JSON text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Raises:
        LexerError: If the source contains characters that don't match
            any token pattern in the JSON grammar.

    Example::

        tokens = tokenize_json('{"name": "Ada", "age": 36}')
        # [Token(LBRACE, '{'), Token(STRING, 'name'), Token(COLON, ':'),
        #  Token(STRING, 'Ada'), Token(COMMA, ','), Token(STRING, 'age'),
        #  Token(COLON, ':'), Token(NUMBER, '36'), Token(RBRACE, '}'),
        #  Token(EOF, '')]
    """
    lexer = create_json_lexer(source)
    return lexer.tokenize()
