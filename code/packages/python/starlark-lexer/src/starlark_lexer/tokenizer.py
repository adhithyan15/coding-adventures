"""Starlark Lexer — tokenizes Starlark source code using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python can tokenize Starlark — or any other
language — simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

Consider the traditional approach to supporting a new language. You would
need to write a brand-new lexer with custom logic for every new token type.
Starlark has operators like ``**`` (exponentiation) and ``//`` (floor
division), augmented assignments like ``+=`` and ``**=``, and four different
string quoting styles with optional prefixes (``r``, ``b``, ``rb``). A
hand-written approach would require adding new methods for each of these.

The grammar-driven approach sidesteps all of that. The ``starlark.tokens``
file declares what tokens Starlark has, and the ``GrammarLexer`` reads those
declarations and compiles them into regex patterns at runtime. No new Python
code is needed for the lexer itself.

Starlark-Specific Features
---------------------------

The ``starlark.tokens`` file uses several features that go beyond basic
token definitions:

1. **Indentation mode** (``mode: indentation``): The lexer tracks leading
   whitespace and emits ``INDENT``, ``DEDENT``, and ``NEWLINE`` tokens
   automatically. This is how Python-style block structure is handled
   without curly braces.

2. **Skip patterns** (``skip:``): Comments (``# ...``) and inline
   whitespace are matched and discarded without producing tokens. This
   keeps the token stream clean — the parser never sees whitespace.

3. **Type aliases** (``-> TYPE``): Multiple regex patterns can emit the
   same token type. For example, all eight string quoting styles
   (``STRING_DQ``, ``STRING_SQ``, ``STRING_RAW_DQ``, etc.) emit
   ``STRING``. This simplifies the grammar — it just says ``STRING``
   instead of listing every variant.

4. **Reserved keywords** (``reserved:``): Words like ``class``, ``import``,
   and ``while`` are legal Python identifiers but illegal in Starlark.
   If the lexer encounters one, it raises an error immediately instead
   of silently misinterpreting the code.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_starlark_lexer(source)`` — creates a ``GrammarLexer`` configured
  for Starlark. Use this when you want to control the tokenization process
  yourself (e.g., for streaming or incremental tokenization).

- ``tokenize_starlark(source)`` — the all-in-one function. Pass in Starlark
  source code, get back a list of tokens. This is the function most callers
  want.

Both functions handle locating and parsing the ``starlark.tokens`` file
automatically.

Pre-compiled Grammar
---------------------

The ``starlark.tokens`` file is compiled ahead of time by the
``grammar-tools`` compiler into ``starlark_lexer/_grammar.py``, which
embeds the ``TokenGrammar`` as native Python data structures. This module
imports ``TOKEN_GRAMMAR`` from it directly — no file I/O or grammar
parsing happens at runtime, and the package works correctly when
installed as a site-package (it does not depend on the
``code/grammars/`` directory existing on disk).

To regenerate after editing ``code/grammars/starlark/starlark.tokens``:

    grammar-tools compile-tokens code/grammars/starlark/starlark.tokens \\
        -o code/packages/python/starlark-lexer/src/starlark_lexer/_grammar.py
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from starlark_lexer._grammar import TOKEN_GRAMMAR


def create_starlark_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Starlark source code.

    This function uses the pre-compiled ``TOKEN_GRAMMAR`` (from
    ``starlark_lexer._grammar``) to create a ``GrammarLexer`` ready to
    tokenize the given source code. No file I/O is performed.

    Use this when you want access to the lexer object itself — for example,
    to inspect its internal state or to integrate with a custom pipeline.
    For most use cases, ``tokenize_starlark()`` is simpler.

    Args:
        source: The Starlark source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with Starlark token
        definitions. Call ``.tokenize()`` on it to get the token list.

    Example::

        lexer = create_starlark_lexer('print("hello")')
        tokens = lexer.tokenize()
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_starlark(source: str) -> list[Token]:
    """Tokenize Starlark source code and return a list of tokens.

    This is the main entry point for the Starlark lexer. Pass in a string
    of Starlark source code, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    Because Starlark uses indentation mode, the returned token list will
    include synthetic tokens that do not correspond to literal characters
    in the source:

    - ``NEWLINE`` — marks the end of a logical line
    - ``INDENT`` — marks an increase in indentation level
    - ``DEDENT`` — marks a decrease in indentation level

    These tokens are suppressed inside brackets (``()``, ``[]``, ``{}``),
    which allows multi-line expressions like::

        result = func(
            arg1,
            arg2,
        )

    to be tokenized without spurious INDENT/DEDENT/NEWLINE tokens.

    The function handles all the setup internally: creating the lexer from
    the pre-compiled grammar and running the tokenization.

    Args:
        source: The Starlark source code to tokenize.

    Returns:
        A list of ``Token`` objects representing the lexical structure
        of the input. The last token is always ``Token(EOF, ...)``.

    Raises:
        LexerError: If the source contains characters that don't match
            any token pattern in the Starlark grammar, or if a reserved
            keyword is encountered.

    Example::

        tokens = tokenize_starlark('x = 1 + 2')
        # [Token(NAME, 'x', 1:1), Token(EQUALS, '=', 1:3),
        #  Token(INT, '1', 1:5), Token(PLUS, '+', 1:7),
        #  Token(INT, '2', 1:9), Token(NEWLINE, ...), Token(EOF, '', ...)]
    """
    lexer = create_starlark_lexer(source)
    return lexer.tokenize()
