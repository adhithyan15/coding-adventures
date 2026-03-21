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

Locating the Grammar File
--------------------------

The ``json.tokens`` file lives in the ``code/grammars/`` directory at the
root of the coding-adventures repository. We locate it relative to this
module's file path using ``pathlib.Path``::

    tokenizer.py
    └── json_lexer/        (parent)
        └── src/           (parent)
            └── json-lexer/  (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── json.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# We navigate from this file's location up to the repository root's
# grammars/ directory. The path is:
#   src/json_lexer/tokenizer.py -> src/json_lexer -> src -> json-lexer
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
JSON_TOKENS_PATH = GRAMMAR_DIR / "json.tokens"


def create_json_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for JSON text.

    This function reads the ``json.tokens`` file, parses it into a
    ``TokenGrammar``, and creates a ``GrammarLexer`` ready to tokenize
    the given source text.

    Args:
        source: The JSON text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with JSON token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Raises:
        FileNotFoundError: If the ``json.tokens`` file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

    Example::

        lexer = create_json_lexer('{"key": "value"}')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(JSON_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


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
        FileNotFoundError: If the ``json.tokens`` file cannot be found.
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
