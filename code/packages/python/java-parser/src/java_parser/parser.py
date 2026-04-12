"""Java Parser — parses Java source code into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the Java lexer: the *same* parser
engine that handles Python or JavaScript can handle Java — just swap the
``.grammar`` file.

Java's grammar includes constructs that Python and JavaScript do not have
in the same form — like explicit type declarations (``int x = 1;``),
access modifiers (``public``, ``private``), and checked exceptions
(``throws IOException``). The grammar-driven approach handles all of these
through grammar rules rather than hard-coded parser logic.

The pipeline is:

1. Read ``java{version}.tokens`` -> build ``TokenGrammar`` -> ``GrammarLexer`` -> tokens
2. Read ``java{version}.grammar`` -> build ``ParserGrammar`` -> ``GrammarParser`` -> AST

Version Support
---------------

This module supports key Java versions tracked by the repo. Pass the
``version`` argument to select a specific version's grammar:

- ``"1.0"``  — Java 1.0 (January 1996)
- ``"1.1"``  — Java 1.1 (February 1997)
- ``"1.4"``  — Java 1.4 (February 2002)
- ``"5"``    — Java 5 (September 2004)
- ``"7"``    — Java 7 (July 2011)
- ``"8"``    — Java 8 (March 2014)
- ``"10"``   — Java 10 (March 2018)
- ``"14"``   — Java 14 (March 2020)
- ``"17"``   — Java 17 (September 2021)
- ``"21"``   — Java 21 (September 2023)

When no ``version`` is given, the default Java 21 grammar is used
(the latest version).

Locating the Grammar Files
--------------------------

Grammar files live in ``code/grammars/java/`` at the repository root::

    parser.py
    └── java_parser/      (parent)
        └── src/          (parent)
            └── java-parser/  (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── java/
                                    ├── java1.0.grammar
                                    ├── java1.1.grammar
                                    ├── java1.4.grammar
                                    ├── java5.grammar
                                    ├── java7.grammar
                                    ├── java8.grammar
                                    ├── java10.grammar
                                    ├── java14.grammar
                                    ├── java17.grammar
                                    └── java21.grammar   ← default
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from java_lexer import tokenize_java
from java_lexer.tokenizer import DEFAULT_VERSION, VALID_VERSIONS
from lang_parser import ASTNode, GrammarParser

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

_GRAMMAR_ROOT = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"


def _resolve_grammar_path(version: str | None) -> Path:
    """Return the Path to the correct .grammar file for the requested version.

    ``version=None`` (or ``""``) loads the default ``java21.grammar``
    file — the latest Java grammar.  Named versions load the corresponding
    versioned file from ``grammars/java/``.

    Args:
        version: One of ``"1.0"``, ``"1.1"``, ``"1.4"``, ``"5"``, ``"7"``,
            ``"8"``, ``"10"``, ``"14"``, ``"17"``, ``"21"``, ``None``, or
            ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.grammar`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized Java version.
    """
    if not version:
        return _GRAMMAR_ROOT / "java" / f"java{DEFAULT_VERSION}.grammar"
    if version not in VALID_VERSIONS:
        raise ValueError(
            f"Unknown Java version {version!r}. "
            f"Valid versions: {sorted(VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "java" / f"java{version}.grammar"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_java_parser(
    source: str, version: str | None = None
) -> GrammarParser:
    """Create a ``GrammarParser`` configured for a specific Java version.

    Args:
        source: The Java source code to parse.
        version: Optional Java version string — ``"1.0"`` through
            ``"21"``.  When omitted (or ``None`` / ``""``), the default
            Java 21 grammar is used.

    Returns:
        A ``GrammarParser`` instance configured with the selected grammar
        rules.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        parser = create_java_parser('public class Hello { }')
        ast = parser.parse()

        parser = create_java_parser('var x = 1;', '10')
        ast = parser.parse()
    """
    tokens = tokenize_java(source, version)
    grammar_path = _resolve_grammar_path(version)
    grammar = parse_parser_grammar(grammar_path.read_text(encoding="utf-8"))
    return GrammarParser(tokens, grammar)


def parse_java(source: str, version: str | None = None) -> ASTNode:
    """Parse Java source code and return an AST.

    This is the main entry point for the Java parser. Pass in a string
    of Java source code, and get back an ``ASTNode`` representing the
    root ``program`` node.

    Args:
        source: The Java source code to parse.
        version: Optional Java version string — ``"1.0"`` through
            ``"21"``.  When omitted (or ``None`` / ``""``), the default
            Java 21 grammar is used.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (Java 21) grammar
        ast = parse_java('public class Hello { }')

        # Java 8 — lambdas, streams
        ast = parse_java('int x = 1;', '8')

        # Java 10 — var keyword
        ast = parse_java('var x = 1;', '10')
    """
    parser = create_java_parser(source, version)
    return parser.parse()
