"""CSS Lexer — tokenizes CSS text using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``css.tokens`` file from the ``code/grammars/`` directory and creates a
lexer configured for CSS tokenization.

CSS (Cascading Style Sheets) is significantly more complex to tokenize than
JSON. The key challenges that this grammar exercises:

- **Compound tokens**: ``10px`` is a single DIMENSION token, not NUMBER + IDENT.
  This requires careful priority ordering (DIMENSION before NUMBER).
- **Function tokens**: ``rgb(`` is a single FUNCTION token. The opening paren
  is part of the token. Must come before IDENT in the priority order.
- **Hash disambiguation**: ``#fff`` (color) and ``#header`` (ID selector) are
  both HASH tokens. The grammar disambiguates by context.
- **At-keywords**: ``@media``, ``@import`` are single AT_KEYWORD tokens.
- **Custom properties**: ``--main-color`` is a CUSTOM_PROPERTY token, not
  two minus signs + IDENT.
- **Error tokens**: ``BAD_STRING`` for unclosed strings, ``BAD_URL`` for
  unclosed URL functions.
- **CSS escapes**: Escape processing is disabled (``escapes: none``) because
  CSS uses hex escapes (``\\26`` for ``&``) that differ from JSON's format.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_css_lexer(source)`` — creates a ``GrammarLexer`` configured for
  CSS. Use this when you want to control the tokenization process yourself.
- ``tokenize_css(source)`` — the all-in-one function. Pass in CSS text,
  get back a list of tokens. This is the function most callers want.

Grammar Source
--------------

The ``css.tokens`` file lives in the ``code/grammars/css/`` directory at
the root of the coding-adventures repository, but this package does not
read it at runtime. It is pre-compiled into ``css_lexer._grammar`` (see
that module's header for regeneration instructions), so tokenizing CSS
requires no file I/O and works correctly in an installed package that
does not ship the monorepo's ``code/grammars/`` directory.
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from css_lexer._grammar import TOKEN_GRAMMAR

# ---------------------------------------------------------------------------
# Grammar Source
# ---------------------------------------------------------------------------
#
# The CSS token grammar (``code/grammars/css/css.tokens``) is pre-compiled
# into ``css_lexer._grammar`` — see that module's header for regeneration
# instructions. Importing the compiled ``TOKEN_GRAMMAR`` constant avoids
# file I/O and runtime parsing on every call.
# ---------------------------------------------------------------------------


def create_css_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for CSS text.

    Uses the pre-compiled ``TOKEN_GRAMMAR`` constant — no file I/O.

    Args:
        source: The CSS text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with CSS token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Example::

        lexer = create_css_lexer('h1 { color: red; }')
        tokens = lexer.tokenize()
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_css(source: str) -> list[Token]:
    """Tokenize CSS text and return a list of tokens.

    This is the main entry point for the CSS lexer. Pass in a string of
    CSS text, and get back a flat list of ``Token`` objects. The list
    always ends with an ``EOF`` token.

    The token types you will see include:

    - **STRING** — a quoted string (quotes stripped, escapes preserved raw)
    - **NUMBER** — an integer or floating-point number
    - **DIMENSION** — a number with a unit suffix (e.g., ``10px``, ``2em``)
    - **PERCENTAGE** — a number with ``%`` (e.g., ``50%``)
    - **HASH** — ``#`` followed by name characters (e.g., ``#fff``, ``#header``)
    - **AT_KEYWORD** — ``@`` followed by identifier (e.g., ``@media``)
    - **FUNCTION** — identifier + ``(`` (e.g., ``rgb(``, ``calc(``)
    - **URL_TOKEN** — ``url(unquoted-content)``
    - **CUSTOM_PROPERTY** — ``--variable-name``
    - **IDENT** — an identifier (e.g., ``color``, ``-webkit-transform``)
    - **UNICODE_RANGE** — e.g., ``U+0025-00FF``
    - Operators: **COLON_COLON**, **TILDE_EQUALS**, etc.
    - Delimiters: **LBRACE**, **RBRACE**, **SEMICOLON**, **COLON**, etc.
    - Legacy: **CDO** (``<!--``), **CDC** (``-->``)
    - Error tokens: **BAD_STRING**, **BAD_URL**
    - **EOF** — end of input

    Args:
        source: The CSS text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Raises:
        FileNotFoundError: If the ``css.tokens`` file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in the CSS grammar.

    Example::

        tokens = tokenize_css('h1 { color: red; }')
        # [Token(IDENT, 'h1'), Token(LBRACE, '{'), Token(IDENT, 'color'),
        #  Token(COLON, ':'), Token(IDENT, 'red'), Token(SEMICOLON, ';'),
        #  Token(RBRACE, '}'), Token(EOF, '')]
    """
    lexer = create_css_lexer(source)
    return lexer.tokenize()
