"""TOML Lexer — tokenizes TOML text using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``toml.tokens`` file from the ``code/grammars/`` directory and creates a
lexer configured for TOML tokenization.

TOML (Tom's Obvious Minimal Language, v1.0.0) sits between JSON and a full
programming language in complexity. Unlike JSON, TOML has:

- **Newline sensitivity.** Key-value pairs are delimited by newlines, so the
  lexer emits NEWLINE tokens instead of silently consuming all whitespace.
  Only spaces and tabs are skipped; ``\\n`` becomes a token. This is similar
  to how Python treats newlines, except TOML has no indentation significance.

- **Multiple string types.** TOML has four kinds of strings:

  ============== ========= ======== ===========
  String Type     Quotes   Escapes  Multi-line
  ============== ========= ======== ===========
  Basic           ``"``    Yes      No
  ML Basic        ``\"\"\"``  Yes   Yes
  Literal         ``'``    No       No
  ML Literal      ``'''``  No       Yes
  ============== ========= ======== ===========

  The lexer emits different token types for each (BASIC_STRING,
  ML_BASIC_STRING, LITERAL_STRING, ML_LITERAL_STRING) so the parser can
  apply the correct escape processing rules.

- **Date/time literals.** TOML natively supports ISO 8601 dates and times as
  first-class values. The four date/time token types are:

  - OFFSET_DATETIME — ``1979-05-27T07:32:00Z`` (with timezone)
  - LOCAL_DATETIME  — ``1979-05-27T07:32:00`` (no timezone)
  - LOCAL_DATE      — ``1979-05-27`` (date only)
  - LOCAL_TIME      — ``07:32:00`` (time only)

  These must be matched *before* bare keys and integers because a date like
  ``1979-05-27`` would otherwise be split into ``1979`` (INTEGER), ``-``
  (part of another token), ``05`` (INTEGER), etc.

- **Comments.** Lines starting with ``#`` (or inline ``# ...``) are comments.
  The lexer skips them, but importantly does NOT consume the trailing newline
  — that newline becomes a NEWLINE token the grammar needs.

- **Bare keys.** Unquoted key names like ``server``, ``my-key``, or
  ``port_number``. These are composed of ASCII letters, digits, dashes, and
  underscores. Because the BARE_KEY pattern is so broad (it would match
  ``true``, ``42``, ``inf``, and even dates), it is defined *last* in the
  grammar — the first-match-wins rule ensures specific patterns win.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_toml_lexer(source)`` — creates a ``GrammarLexer`` configured for
  TOML. Use this when you want to control the tokenization process yourself.
- ``tokenize_toml(source)`` — the all-in-one function. Pass in TOML text,
  get back a list of tokens. This is the function most callers want.

The 20 Token Types
------------------

TOML has significantly more token types than JSON (which has only 11):

**Strings (4):** BASIC_STRING, ML_BASIC_STRING, LITERAL_STRING, ML_LITERAL_STRING
**Numbers (2):** INTEGER, FLOAT
**Booleans (2):** TRUE, FALSE
**Date/Times (4):** OFFSET_DATETIME, LOCAL_DATETIME, LOCAL_DATE, LOCAL_TIME
**Keys (1):** BARE_KEY
**Delimiters (7):** EQUALS, DOT, COMMA, LBRACKET, RBRACKET, LBRACE, RBRACE

Plus the two tokens every grammar produces: NEWLINE and EOF.

Locating the Grammar File
--------------------------

The ``toml.tokens`` file lives in the ``code/grammars/`` directory at the
root of the coding-adventures repository. We locate it relative to this
module's file path using ``pathlib.Path``::

    tokenizer.py
    └── toml_lexer/        (parent)
        └── src/           (parent)
            └── toml-lexer/  (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── toml.tokens
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
#   src/toml_lexer/tokenizer.py -> src/toml_lexer -> src -> toml-lexer
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
TOML_TOKENS_PATH = GRAMMAR_DIR / "toml.tokens"


def create_toml_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for TOML text.

    This function reads the ``toml.tokens`` file, parses it into a
    ``TokenGrammar``, and creates a ``GrammarLexer`` ready to tokenize
    the given source text.

    Why a factory function instead of a class?
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    Because the lexer *engine* already exists — ``GrammarLexer`` does all the
    work. This function's only job is to load the right ``.tokens`` file and
    wire it up. A class would add ceremony without adding capability.

    Args:
        source: The TOML text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with TOML token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Raises:
        FileNotFoundError: If the ``toml.tokens`` file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

    Example::

        lexer = create_toml_lexer('name = "TOML"')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(TOML_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_toml(source: str) -> list[Token]:
    """Tokenize TOML text and return a list of tokens.

    This is the main entry point for the TOML lexer. Pass in a string of
    TOML text, and get back a flat list of ``Token`` objects. The list
    always ends with an ``EOF`` token.

    The 20 token types you will see are:

    **Strings:**

    - **BASIC_STRING** — ``"hello"`` (double-quoted, supports escapes)
    - **ML_BASIC_STRING** — ``\"\"\"multi\\nline\"\"\"`` (triple-double-quoted)
    - **LITERAL_STRING** — ``'hello'`` (single-quoted, no escapes)
    - **ML_LITERAL_STRING** — ``'''multi\\nline'''`` (triple-single-quoted)

    **Numbers:**

    - **INTEGER** — decimal (``42``), hex (``0xFF``), octal (``0o77``),
      binary (``0b1010``), with optional sign and underscores
    - **FLOAT** — decimal (``3.14``), scientific (``1e10``), special
      (``inf``, ``nan``), with optional sign and underscores

    **Booleans:**

    - **TRUE** — the literal ``true``
    - **FALSE** — the literal ``false``

    **Date/Times:**

    - **OFFSET_DATETIME** — ``1979-05-27T07:32:00Z``
    - **LOCAL_DATETIME** — ``1979-05-27T07:32:00``
    - **LOCAL_DATE** — ``1979-05-27``
    - **LOCAL_TIME** — ``07:32:00``

    **Keys:**

    - **BARE_KEY** — ``server``, ``my-key``, ``port_number``

    **Delimiters:**

    - **EQUALS** — ``=``
    - **DOT** — ``.``
    - **COMMA** — ``,``
    - **LBRACKET** / **RBRACKET** — ``[`` and ``]``
    - **LBRACE** / **RBRACE** — ``{`` and ``}``

    **Structural:**

    - **NEWLINE** — emitted for each ``\\n`` (TOML is newline-sensitive)
    - **EOF** — end of input

    Args:
        source: The TOML text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Raises:
        FileNotFoundError: If the ``toml.tokens`` file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in the TOML grammar.

    Example::

        tokens = tokenize_toml('name = "TOML"\\nversion = "1.0.0"')
        # [Token(BARE_KEY, 'name'), Token(EQUALS, '='),
        #  Token(BASIC_STRING, 'TOML'), Token(NEWLINE, '\\\\n'),
        #  Token(BARE_KEY, 'version'), Token(EQUALS, '='),
        #  Token(BASIC_STRING, '1.0.0'), Token(NEWLINE, '\\\\n'),
        #  Token(EOF, '')]
    """
    lexer = create_toml_lexer(source)
    return lexer.tokenize()
