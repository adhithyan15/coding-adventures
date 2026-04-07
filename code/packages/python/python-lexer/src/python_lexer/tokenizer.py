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

Grammar Caching
----------------

Parsing a ``.tokens`` file involves reading from disk and running the
grammar parser. To avoid this overhead on every call, we cache the parsed
``TokenGrammar`` object per version in a module-level dictionary.

The cache is safe for concurrent use patterns because:
1. ``TokenGrammar`` objects are effectively immutable after construction.
2. The worst case with a race is that two threads parse the same grammar
   simultaneously and one write overwrites the other — the result is
   identical, so no corruption occurs.

Locating the Grammar Files
---------------------------

The grammar files live in the ``code/grammars/python/`` directory at the
root of the coding-adventures repository. We locate them relative to this
module's file path using ``pathlib.Path``. The path traversal is::

    tokenizer.py
    └── python_lexer/       (parent)
        └── src/            (parent)
            └── python-lexer/ (parent)
                └── python/     (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── python/
                                    └── python3.12.tokens

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

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

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
# Grammar File Location
# ---------------------------------------------------------------------------
#
# We navigate from this file's location up to the repository root's
# grammars/python/ directory. The path is:
#   src/python_lexer/tokenizer.py -> src/python_lexer -> src ->
#   python-lexer -> python -> packages -> code -> code/grammars/python
#
# Using Path(__file__) makes this work regardless of the current working
# directory, which is important for testing and for use as an installed
# package.
# ---------------------------------------------------------------------------

_GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "python"


def _grammar_path(version: str) -> Path:
    """Return the path to the ``.tokens`` file for the given version.

    Args:
        version: A Python version string like ``"3.12"`` or ``"2.7"``.

    Returns:
        The ``Path`` to the corresponding grammar file, e.g.
        ``.../code/grammars/python/python3.12.tokens``.

    Example::

        _grammar_path("3.12")
        # → Path(".../code/grammars/python/python3.12.tokens")
    """
    return _GRAMMAR_DIR / f"python{version}.tokens"


# ---------------------------------------------------------------------------
# Grammar Cache
# ---------------------------------------------------------------------------
#
# Parsed TokenGrammar objects are cached per version string. Once a grammar
# is parsed, it is reused for all subsequent calls with that version.
#
# The cache is a simple dictionary. In a multi-threaded environment, the
# worst case is that two threads parse the same grammar simultaneously and
# one overwrites the other's cache entry — but both produce identical
# TokenGrammar objects, so this is harmless.
# ---------------------------------------------------------------------------

_grammar_cache: dict[str, object] = {}


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_python_lexer(source: str, version: str = DEFAULT_VERSION) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for the given Python version.

    This function reads the appropriate ``pythonX.Y.tokens`` grammar file,
    parses it into a ``TokenGrammar``, and creates a ``GrammarLexer`` ready
    to tokenize the given source code.

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
        FileNotFoundError: If the grammar file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

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

    # Check the cache first (fast path).
    if version in _grammar_cache:
        grammar = _grammar_cache[version]
        return GrammarLexer(source, grammar)

    # Cache miss — read and parse the grammar file.
    path = _grammar_path(version)
    grammar = parse_token_grammar(path.read_text())

    # Store in cache for future calls.
    _grammar_cache[version] = grammar

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
