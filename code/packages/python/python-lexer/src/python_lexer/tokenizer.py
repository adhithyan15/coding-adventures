"""Python Lexer — tokenizes Python source code using versioned grammar files.

This module is a thin wrapper around the generic ``GrammarLexer``. It supports
multiple Python versions by loading version-specific ``.tokens`` grammar files
from the ``code/grammars/python/`` directory.

Versioned Grammar Loading
--------------------------

Each Python version has its own grammar file:

    python2.7.tokens   — Python 2.7 (legacy, ``print`` is a keyword)
    python3.0.tokens   — Python 3.0 (``print`` became a function)
    python3.6.tokens   — Python 3.6 (f-strings, underscores in numbers)
    python3.8.tokens   — Python 3.8 (walrus operator ``:=``)
    python3.10.tokens  — Python 3.10 (``match``/``case`` soft keywords)
    python3.12.tokens  — Python 3.12 (``type`` soft keyword)

The ``version`` parameter selects which grammar to load. When omitted, it
defaults to ``"3.12"`` (the latest supported version).

Pre-compiled Grammars
----------------------

Each version's ``.tokens`` file is compiled ahead of time by the
``grammar-tools`` compiler into a ``_grammar_<version>.py`` module that
embeds the ``TokenGrammar`` as native Python data structures. This package
imports all of them and looks them up in the ``_TOKEN_GRAMMARS`` dict —
no file I/O or grammar parsing happens at runtime, and the package works
correctly when installed as a site-package (it does not depend on the
``code/grammars/`` directory existing on disk).

What This Module Provides
--------------------------

Two convenience functions and two constants:

- ``tokenize_python(source, version)`` — the all-in-one function. Pass in
  Python source code and optionally a version string, get back a list of
  tokens.

- ``create_python_lexer(source, version)`` — creates a ``GrammarLexer``
  configured for the given Python version. Use this when you want to control
  the tokenization process yourself.

- ``DEFAULT_VERSION`` — the default Python version (``"3.12"``).

- ``SUPPORTED_VERSIONS`` — the list of all supported version strings.
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from python_lexer import (
    _grammar_2_7,
    _grammar_3_0,
    _grammar_3_6,
    _grammar_3_8,
    _grammar_3_10,
    _grammar_3_12,
)

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

DEFAULT_VERSION: str = "3.12"
"""The Python version used when no version is specified.

We default to the latest grammar we have — Python 3.12. This includes
all modern features: f-strings, walrus operator, match/case soft keywords,
and the type alias soft keyword.
"""

SUPPORTED_VERSIONS: list[str] = ["2.7", "3.0", "3.6", "3.8", "3.10", "3.12"]
"""All Python versions with grammar files.

Each version has a corresponding ``pythonX.Y.tokens`` file in the grammars
directory. The list is ordered chronologically from oldest to newest.
"""


# ---------------------------------------------------------------------------
# Grammar Lookup
# ---------------------------------------------------------------------------
#
# Each Python version's TokenGrammar is pre-compiled into its own
# ``_grammar_<version>.py`` module by the grammar-tools compiler (see
# ``code/programs/python/grammar-tools``). Importing these modules embeds
# the grammars as native Python data structures, so no file I/O or grammar
# parsing happens at runtime:
#
#   1. No file I/O at startup — no open(), read(), or path traversal.
#   2. No runtime grammar parsing overhead.
#   3. The package is self-contained — it works correctly when installed
#      as a site-package in any venv, not just when run from the source
#      tree (a published PyPI package does not ship ``code/grammars/``).
#
# To regenerate after editing a ``code/grammars/python/pythonX.Y.tokens``
# file:
#   grammar-tools compile-tokens code/grammars/python/pythonX.Y.tokens \
#       -o code/packages/python/python-lexer/src/python_lexer/_grammar_X_Y.py
# ---------------------------------------------------------------------------

_TOKEN_GRAMMARS: dict[str, object] = {
    "2.7": _grammar_2_7.TOKEN_GRAMMAR,
    "3.0": _grammar_3_0.TOKEN_GRAMMAR,
    "3.6": _grammar_3_6.TOKEN_GRAMMAR,
    "3.8": _grammar_3_8.TOKEN_GRAMMAR,
    "3.10": _grammar_3_10.TOKEN_GRAMMAR,
    "3.12": _grammar_3_12.TOKEN_GRAMMAR,
}


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_python_lexer(source: str, version: str = DEFAULT_VERSION) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for the given Python version.

    This function looks up the pre-compiled ``TokenGrammar`` for the given
    version from ``_TOKEN_GRAMMARS`` and creates a ``GrammarLexer`` ready to
    tokenize the given source code. No file I/O is performed.

    Use this when you want access to the lexer object itself — for example,
    to inspect its internal state or to integrate with a custom pipeline.
    For most use cases, ``tokenize_python()`` is simpler.

    Args:
        source: The Python source code to tokenize.
        version: The Python version to use for grammar selection. Must be
            one of the values in ``SUPPORTED_VERSIONS``. Defaults to
            ``DEFAULT_VERSION`` (``"3.12"``).

    Returns:
        A ``GrammarLexer`` instance configured with the appropriate Python
        token definitions. Call ``.tokenize()`` on it to get the token list.

    Raises:
        ValueError: If ``version`` is not in ``SUPPORTED_VERSIONS``.

    Example::

        lexer = create_python_lexer('x = 1 + 2', version="3.12")
        tokens = lexer.tokenize()

        # Python 2.7 — print is a keyword
        lexer = create_python_lexer('print "hello"', version="2.7")
        tokens = lexer.tokenize()
    """
    if version not in SUPPORTED_VERSIONS:
        raise ValueError(
            f"Unsupported Python version: {version!r}. "
            f"Supported versions: {', '.join(SUPPORTED_VERSIONS)}"
        )

    grammar = _TOKEN_GRAMMARS[version]
    return GrammarLexer(source, grammar)


def tokenize_python(
    source: str,
    version: str = DEFAULT_VERSION,
) -> list[Token]:
    """Tokenize Python source code and return a list of tokens.

    This is the main entry point for the Python lexer. Pass in a string
    of Python source code and optionally a version string, and get back
    a flat list of ``Token`` objects. The list always ends with an ``EOF``
    token.

    Because Python uses indentation mode, the returned token list will
    include synthetic tokens that do not correspond to literal characters
    in the source:

    - ``NEWLINE`` — marks the end of a logical line
    - ``INDENT`` — marks an increase in indentation level
    - ``DEDENT`` — marks a decrease in indentation level

    These tokens are suppressed inside brackets (``()``, ``[]``, ``{}``),
    which allows multi-line expressions.

    The function handles all the setup internally: selecting the grammar
    file for the given version, parsing it (with caching), creating the
    lexer, and running the tokenization.

    Args:
        source: The Python source code to tokenize.
        version: The Python version to use for grammar selection. Must be
            one of the values in ``SUPPORTED_VERSIONS``. Defaults to
            ``DEFAULT_VERSION`` (``"3.12"``).

    Returns:
        A list of ``Token`` objects representing the lexical structure
        of the input. The last token is always ``Token(EOF, ...)``.

    Raises:
        ValueError: If ``version`` is not in ``SUPPORTED_VERSIONS``.
        FileNotFoundError: If the grammar file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in the Python grammar.

    Example::

        tokens = tokenize_python('x = 1 + 2')
        # [Token(NAME, 'x', 1:1), Token(EQUALS, '=', 1:3),
        #  Token(INT, '1', 1:5), Token(PLUS, '+', 1:7),
        #  Token(INT, '2', 1:9), Token(NEWLINE, ...), Token(EOF, '', ...)]

        # Use a specific version
        tokens = tokenize_python('print "hello"', version="2.7")
    """
    python_lexer = create_python_lexer(source, version)
    return python_lexer.tokenize()
