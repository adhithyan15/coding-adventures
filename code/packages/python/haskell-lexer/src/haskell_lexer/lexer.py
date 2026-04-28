"""Haskell Lexer â€” tokenizes Haskell source code using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python or HaskellScript can tokenize Haskell â€” or any
other language â€” simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

Haskell has tokens that differ from Python and HaskellScript â€” like access
modifiers (``public``, ``private``, ``protected``), type keywords
(``int``, ``boolean``, ``void``), and object-oriented constructs
(``class``, ``interface``, ``extends``, ``implements``). The grammar-driven
approach handles all of these without any new tokenization code: they are
declared in the ``.tokens`` file, and the ``GrammarLexer`` compiles them
into regex patterns at runtime.

Version Support
---------------

This module supports key Haskell versions tracked by the repo. Pass the
``version`` argument to select a specific version's grammar:

- ``"1.0"``  â€” Haskell 1.0 (January 1996): the original release. The
  foundation: classes, interfaces, exceptions, threads.
- ``"1.1"``  â€” Haskell 1.1 (February 1997): inner classes, reflection, JDBC,
  HaskellBeans.
- ``"1.4"``  â€” Haskell 1.4 (February 2002): ``assert`` keyword, regex, NIO,
  logging, XML parsing.
- ``"5"``    â€” Haskell 5 (September 2004): generics, ``enum``, annotations
  (``@Override``), autoboxing, varargs, enhanced ``for`` loop.
- ``"7"``    â€” Haskell 7 (July 2011): try-with-resources, diamond operator
  (``<>``), multi-catch, strings in ``switch``.
- ``"8"``    â€” Haskell 8 (March 2014): lambdas (``->``), streams, default
  methods, ``Optional``, method references (``::``).
- ``"10"``   â€” Haskell 10 (March 2018): local variable type inference
  (``var``).
- ``"14"``   â€” Haskell 14 (March 2020): ``switch`` expressions (``yield``),
  records (preview), helpful NullPointerExceptions.
- ``"17"``   â€” Haskell 17 (September 2021): sealed classes (``sealed``,
  ``permits``), pattern matching for ``instanceof``.
- ``"21"``   â€” Haskell 21 (September 2023): virtual threads, pattern matching
  for ``switch``, record patterns, string templates (preview).

When no ``version`` is given (or ``None`` / ``""``), the latest version
(Haskell 21) is used as the default.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_haskell_lexer(source, version)`` â€” creates a ``GrammarLexer``
  configured for the requested Haskell version.

- ``tokenize_haskell(source, version)`` â€” the all-in-one function. Pass in
  Haskell source code, get back a list of tokens.

Locating the Grammar Files
--------------------------

Grammar files live in ``code/grammars/haskell/`` at the repository root::

    lexer.py
    â””â”€â”€ haskell_lexer/       (parent)
        â””â”€â”€ src/          (parent)
            â””â”€â”€ haskell-lexer/   (parent)
                â””â”€â”€ python/       (parent)
                    â””â”€â”€ packages/ (parent)
                        â””â”€â”€ code/     (parent)
                            â””â”€â”€ grammars/
                                â””â”€â”€ haskell/
                                    â”œâ”€â”€ haskell1.0.tokens
                                    â”œâ”€â”€ haskell1.1.tokens
                                    â”œâ”€â”€ haskell1.4.tokens
                                    â”œâ”€â”€ haskell5.tokens
                                    â”œâ”€â”€ haskell7.tokens
                                    â”œâ”€â”€ haskell8.tokens
                                    â”œâ”€â”€ haskell10.tokens
                                    â”œâ”€â”€ haskell14.tokens
                                    â”œâ”€â”€ haskell17.tokens
                                    â””â”€â”€ haskell21.tokens   â† default
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

_GRAMMAR_ROOT = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"

# The set of valid version strings.  Each maps to a file under
# code/grammars/haskell/haskell<version>.tokens
VALID_VERSIONS = frozenset({
    "1.0", "1.1", "1.2", "1.3", "1.4", "98", "2010",
})

DEFAULT_VERSION = "2010"


def _resolve_tokens_path(version: str | None) -> Path:
    """Return the Path to the correct .tokens file for the requested version.

    ``version=None`` (or ``""``) loads the default ``haskell21.tokens``
    file â€” the latest Haskell grammar.  Named versions load the corresponding
    versioned file from ``grammars/haskell/``.

    Args:
        version: One of ``"1.0"``, ``"1.1"``, ``"1.4"``, ``"5"``, ``"7"``,
            ``"8"``, ``"10"``, ``"14"``, ``"17"``, ``"21"``, ``None``, or
            ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.tokens`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized Haskell version.
    """
    if not version:
        return _GRAMMAR_ROOT / "haskell" / f"haskell{DEFAULT_VERSION}.tokens"
    if version not in VALID_VERSIONS:
        raise ValueError(
            f"Unknown Haskell version {version!r}. "
            f"Valid versions: {sorted(VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "haskell" / f"haskell{version}.tokens"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_haskell_lexer(
    source: str, version: str | None = None
) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for a specific Haskell version.

    Args:
        source: The Haskell source code to tokenize.
        version: Optional Haskell version string â€” ``"1.0"`` through
            ``"21"``.  When omitted (or ``None`` / ``""``), the default
            Haskell 21 grammar is used.

    Returns:
        A ``GrammarLexer`` instance configured with the selected token
        definitions.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        lexer = create_haskell_lexer('public class Hello { }', '17')
        tokens = lexer.tokenize()
    """
    tokens_path = _resolve_tokens_path(version)
    grammar = parse_token_grammar(tokens_path.read_text(encoding="utf-8"))
    return GrammarLexer(source, grammar)


def tokenize_haskell(
    source: str, version: str | None = None
) -> list[Token]:
    """Tokenize Haskell source code and return a list of tokens.

    This is the main entry point for the Haskell lexer. Pass in a string
    of Haskell source code, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    Args:
        source: The Haskell source code to tokenize.
        version: Optional Haskell version string â€” ``"1.0"`` through
            ``"21"``.  When omitted (or ``None`` / ``""``), the default
            Haskell 21 grammar is used.

    Returns:
        A list of ``Token`` objects.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (Haskell 21) grammar
        tokens = tokenize_haskell('public class Hello { }')

        # Haskell 8 â€” lambdas, streams, default methods
        tokens = tokenize_haskell('x -> x * 2', '8')

        # Haskell 5 â€” generics, enums, annotations
        tokens = tokenize_haskell('enum Color { RED, GREEN, BLUE }', '5')

        # Haskell 21 â€” latest: virtual threads, pattern matching
        tokens = tokenize_haskell('sealed interface Shape permits Circle { }', '21')
    """
    lexer = create_haskell_lexer(source, version)
    return lexer.tokenize()

