"""Lisp Lexer — tokenizes Lisp text using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It uses
a pre-compiled ``TOKEN_GRAMMAR`` (see ``lisp_lexer._grammar``) and creates a
lexer configured for Lisp tokenization.

Lisp Tokenization
-----------------

Lisp has a beautifully simple token set compared to languages like CSS.
The key challenge is that Lisp symbols can contain characters that most
languages treat as operators: ``+``, ``-``, ``*``, ``/``, ``=``, ``<``,
``>``, ``!``, ``?``, ``&``.

This means ``+`` is a valid symbol (the addition function), and so are
``define``, ``lambda``, ``car``, etc. The token priority ordering handles
potential ambiguity:

1. ``NUMBER`` (``/-?[0-9]+/``) comes before ``SYMBOL`` — so ``-42`` tokenizes
   as a single NUMBER, not a SYMBOL ``-`` followed by NUMBER ``42``.
2. ``SYMBOL`` includes operator characters in its character class.
3. ``DOT`` is a separate token for dotted pairs: ``(a . b)``.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_lisp_lexer(source)`` — creates a ``GrammarLexer`` configured for
  Lisp. Use this when you want to control the tokenization process yourself.
- ``tokenize_lisp(source)`` — the all-in-one function. Pass in Lisp text,
  get back a list of tokens.

The Lisp token grammar is compiled into ``_grammar.py`` by the grammar-tools
compiler from ``code/grammars/lisp/lisp.tokens``. Importing the pre-built
``TOKEN_GRAMMAR`` constant means no file I/O at startup and no runtime
grammar parsing overhead.

To regenerate after editing ``code/grammars/lisp/lisp.tokens``::

    grammar-tools compile-tokens code/grammars/lisp/lisp.tokens \\
        > code/packages/python/lisp-lexer/src/lisp_lexer/_grammar.py
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from lisp_lexer._grammar import TOKEN_GRAMMAR


def create_lisp_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Lisp text.

    This function uses the pre-compiled ``TOKEN_GRAMMAR`` (from
    ``lisp_lexer._grammar``) to create a ``GrammarLexer`` ready to tokenize
    the given source text. No file I/O is performed.

    Args:
        source: The Lisp text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with Lisp token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Example::

        lexer = create_lisp_lexer('(+ 1 2)')
        tokens = lexer.tokenize()
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_lisp(source: str) -> list[Token]:
    """Tokenize Lisp text and return a list of tokens.

    This is the main entry point for the Lisp lexer. Pass in a string of
    Lisp text, and get back a flat list of ``Token`` objects. The list
    always ends with an ``EOF`` token.

    The token types you will see include:

    - **NUMBER** — an integer literal, possibly negative (e.g., ``42``, ``-7``)
    - **SYMBOL** — an identifier or operator name (e.g., ``define``, ``+``,
      ``factorial``, ``car``)
    - **STRING** — a double-quoted string (e.g., ``"hello"``)
    - **LPAREN** / **RPAREN** — ``(`` and ``)``
    - **QUOTE** — ``'`` (syntactic sugar for ``(quote ...)``)
    - **DOT** — ``.`` (for dotted pairs like ``(a . b)``)
    - **EOF** — end of input

    Whitespace and comments (starting with ``;``) are automatically skipped.

    Args:
        source: The Lisp text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Raises:
        LexerError: If the source contains characters that don't match
            any token pattern in the Lisp grammar.

    Example::

        tokens = tokenize_lisp('(define x 42)')
        # [Token(LPAREN, '('), Token(SYMBOL, 'define'),
        #  Token(SYMBOL, 'x'), Token(NUMBER, '42'),
        #  Token(RPAREN, ')'), Token(EOF, '')]
    """
    lexer = create_lisp_lexer(source)
    return lexer.tokenize()
