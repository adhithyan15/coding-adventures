"""mosaic-lexer — Tokenizes .mosaic source using the grammar-driven lexer.

Mosaic is a component description language for UI. A .mosaic file declares
one UI component with typed data slots and a visual node tree. It compiles
to platform-native code: React TSX, Web Components, SwiftUI, Compose, etc.

Token Types
-----------

The Mosaic token grammar defines the following token types:

- **STRING** — double-quoted string literals, e.g. ``"hello"``
- **DIMENSION** — number with unit suffix, e.g. ``16dp``, ``100%``, ``1.5sp``
- **NUMBER** — bare numeric literal, e.g. ``42``, ``-3.14``
- **COLOR_HEX** — hex color, e.g. ``#fff``, ``#2563eb``, ``#ff000080``
- **NAME** — identifier (may include hyphens), e.g. ``title``, ``avatar-url``
- **KEYWORD** — reserved keyword: ``component``, ``slot``, ``import``, etc.
- **LBRACE** / **RBRACE** — ``{`` / ``}``
- **LANGLE** / **RANGLE** — ``<`` / ``>``
- **COLON** / **SEMICOLON** / **COMMA** / **DOT** / **EQUALS** / **AT**

How It Works
------------

The Mosaic lexer is a thin wrapper around the generic ``GrammarLexer`` from the
``lexer`` package. The embedded ``TOKEN_GRAMMAR`` in ``_grammar.py`` is the
compiled form of ``mosaic.tokens``.

Unlike the Starlark lexer, Mosaic uses the default (non-indentation) mode.
Block structure is delimited by ``{`` / ``}`` braces, so no INDENT/DEDENT
tokens are generated.

Usage::

    from mosaic_lexer import tokenize

    tokens = tokenize('component Label { slot text: text; Text { content: @text; } }')
    for token in tokens:
        print(token)
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from mosaic_lexer._grammar import TOKEN_GRAMMAR

__version__ = "0.1.0"

__all__ = [
    "tokenize",
    "create_lexer",
    "TOKEN_GRAMMAR",
]


def create_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Mosaic source code.

    Use this when you want direct access to the lexer object — for example,
    to inspect token positions or integrate with a custom pipeline. For most
    use cases, ``tokenize()`` is simpler.

    Args:
        source: The Mosaic source code to tokenize.

    Returns:
        A ``GrammarLexer`` ready to tokenize Mosaic. Call ``.tokenize()``
        on it to get the token list.

    Example::

        lexer = create_lexer('component Label {}')
        tokens = lexer.tokenize()
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize(source: str) -> list[Token]:
    """Tokenize Mosaic source code and return a list of tokens.

    This is the main entry point for the Mosaic lexer. Pass in a string of
    Mosaic source code and get back a flat list of ``Token`` objects. The list
    always ends with an ``EOF`` token.

    Mosaic uses brace-delimited block structure (not indentation), so the
    token stream never contains synthetic INDENT/DEDENT/NEWLINE tokens.

    Token stream example for ``component Label { slot text: text; }``::

        KEYWORD('component')  NAME('Label')  LBRACE('{')
        KEYWORD('slot')  NAME('text')  COLON(':')  KEYWORD('text')  SEMICOLON(';')
        RBRACE('}')  EOF

    Args:
        source: The Mosaic source code to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always ``EOF``.

    Raises:
        LexerError: If the source contains characters that do not match
            any token pattern in the Mosaic grammar.

    Example::

        tokens = tokenize('slot title: text;')
        types = [t.type for t in tokens]
        # ['KEYWORD', 'NAME', 'COLON', 'KEYWORD', 'SEMICOLON', 'EOF']
    """
    return create_lexer(source).tokenize()
