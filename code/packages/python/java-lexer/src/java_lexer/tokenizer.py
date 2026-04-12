"""Java Lexer — tokenizes Java source code using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python or JavaScript can tokenize Java — or any
other language — simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

Java has tokens that differ from Python and JavaScript — like access
modifiers (``public``, ``private``, ``protected``), type keywords
(``int``, ``boolean``, ``void``), and object-oriented constructs
(``class``, ``interface``, ``extends``, ``implements``). The grammar-driven
approach handles all of these without any new tokenization code: they are
declared in the ``.tokens`` file, and the ``GrammarLexer`` compiles them
into regex patterns at runtime.

Version Support
---------------

This module supports key Java versions tracked by the repo. Pass the
``version`` argument to select a specific version's grammar:

- ``"1.0"``  — Java 1.0 (January 1996): the original release. The
  foundation: classes, interfaces, exceptions, threads.
- ``"1.1"``  — Java 1.1 (February 1997): inner classes, reflection, JDBC,
  JavaBeans.
- ``"1.4"``  — Java 1.4 (February 2002): ``assert`` keyword, regex, NIO,
  logging, XML parsing.
- ``"5"``    — Java 5 (September 2004): generics, ``enum``, annotations
  (``@Override``), autoboxing, varargs, enhanced ``for`` loop.
- ``"7"``    — Java 7 (July 2011): try-with-resources, diamond operator
  (``<>``), multi-catch, strings in ``switch``.
- ``"8"``    — Java 8 (March 2014): lambdas (``->``), streams, default
  methods, ``Optional``, method references (``::``).
- ``"10"``   — Java 10 (March 2018): local variable type inference
  (``var``).
- ``"14"``   — Java 14 (March 2020): ``switch`` expressions (``yield``),
  records (preview), helpful NullPointerExceptions.
- ``"17"``   — Java 17 (September 2021): sealed classes (``sealed``,
  ``permits``), pattern matching for ``instanceof``.
- ``"21"``   — Java 21 (September 2023): virtual threads, pattern matching
  for ``switch``, record patterns, string templates (preview).

When no ``version`` is given (or ``None`` / ``""``), the latest version
(Java 21) is used as the default.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_java_lexer(source, version)`` — creates a ``GrammarLexer``
  configured for the requested Java version.

- ``tokenize_java(source, version)`` — the all-in-one function. Pass in
  Java source code, get back a list of tokens.

Locating the Grammar Files
--------------------------

Grammar files live in ``code/grammars/java/`` at the repository root::

    tokenizer.py
    └── java_lexer/       (parent)
        └── src/          (parent)
            └── java-lexer/   (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── java/
                                    ├── java1.0.tokens
                                    ├── java1.1.tokens
                                    ├── java1.4.tokens
                                    ├── java5.tokens
                                    ├── java7.tokens
                                    ├── java8.tokens
                                    ├── java10.tokens
                                    ├── java14.tokens
                                    ├── java17.tokens
                                    └── java21.tokens   ← default
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
# code/grammars/java/java<version>.tokens
VALID_VERSIONS = frozenset({
    "1.0", "1.1", "1.4", "5", "7", "8", "10", "14", "17", "21",
})

DEFAULT_VERSION = "21"


def _resolve_tokens_path(version: str | None) -> Path:
    """Return the Path to the correct .tokens file for the requested version.

    ``version=None`` (or ``""``) loads the default ``java21.tokens``
    file — the latest Java grammar.  Named versions load the corresponding
    versioned file from ``grammars/java/``.

    Args:
        version: One of ``"1.0"``, ``"1.1"``, ``"1.4"``, ``"5"``, ``"7"``,
            ``"8"``, ``"10"``, ``"14"``, ``"17"``, ``"21"``, ``None``, or
            ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.tokens`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized Java version.
    """
    if not version:
        return _GRAMMAR_ROOT / "java" / f"java{DEFAULT_VERSION}.tokens"
    if version not in VALID_VERSIONS:
        raise ValueError(
            f"Unknown Java version {version!r}. "
            f"Valid versions: {sorted(VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "java" / f"java{version}.tokens"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_java_lexer(
    source: str, version: str | None = None
) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for a specific Java version.

    Args:
        source: The Java source code to tokenize.
        version: Optional Java version string — ``"1.0"`` through
            ``"21"``.  When omitted (or ``None`` / ``""``), the default
            Java 21 grammar is used.

    Returns:
        A ``GrammarLexer`` instance configured with the selected token
        definitions.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        lexer = create_java_lexer('public class Hello { }', '17')
        tokens = lexer.tokenize()
    """
    tokens_path = _resolve_tokens_path(version)
    grammar = parse_token_grammar(tokens_path.read_text(encoding="utf-8"))
    return GrammarLexer(source, grammar)


def tokenize_java(
    source: str, version: str | None = None
) -> list[Token]:
    """Tokenize Java source code and return a list of tokens.

    This is the main entry point for the Java lexer. Pass in a string
    of Java source code, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    Args:
        source: The Java source code to tokenize.
        version: Optional Java version string — ``"1.0"`` through
            ``"21"``.  When omitted (or ``None`` / ``""``), the default
            Java 21 grammar is used.

    Returns:
        A list of ``Token`` objects.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (Java 21) grammar
        tokens = tokenize_java('public class Hello { }')

        # Java 8 — lambdas, streams, default methods
        tokens = tokenize_java('x -> x * 2', '8')

        # Java 5 — generics, enums, annotations
        tokens = tokenize_java('enum Color { RED, GREEN, BLUE }', '5')

        # Java 21 — latest: virtual threads, pattern matching
        tokens = tokenize_java('sealed interface Shape permits Circle { }', '21')
    """
    lexer = create_java_lexer(source, version)
    return lexer.tokenize()
