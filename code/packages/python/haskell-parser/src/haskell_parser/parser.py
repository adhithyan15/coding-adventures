"""Haskell Parser â€” parses Haskell source code into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the Haskell lexer: the *same* parser
engine that handles Python or HaskellScript can handle Haskell â€” just swap the
``.grammar`` file.

Haskell's grammar includes constructs that Python and HaskellScript do not have
in the same form â€” like explicit type declarations (``int x = 1;``),
access modifiers (``public``, ``private``), and checked exceptions
(``throws IOException``). The grammar-driven approach handles all of these
through grammar rules rather than hard-coded parser logic.

The pipeline is:

1. Read ``haskell{version}.tokens`` -> build ``TokenGrammar`` -> ``GrammarLexer`` -> tokens
2. Read ``haskell{version}.grammar`` -> build ``ParserGrammar`` -> ``GrammarParser`` -> AST

Version Support
---------------

This module supports key Haskell versions tracked by the repo. Pass the
``version`` argument to select a specific version's grammar:

- ``"1.0"``  â€” Haskell 1.0 (January 1996)
- ``"1.1"``  â€” Haskell 1.1 (February 1997)
- ``"1.4"``  â€” Haskell 1.4 (February 2002)
- ``"5"``    â€” Haskell 5 (September 2004)
- ``"7"``    â€” Haskell 7 (July 2011)
- ``"8"``    â€” Haskell 8 (March 2014)
- ``"10"``   â€” Haskell 10 (March 2018)
- ``"14"``   â€” Haskell 14 (March 2020)
- ``"17"``   â€” Haskell 17 (September 2021)
- ``"21"``   â€” Haskell 21 (September 2023)

When no ``version`` is given, the default Haskell 21 grammar is used
(the latest version).

Locating the Grammar Files
--------------------------

Grammar files live in ``code/grammars/haskell/`` at the repository root::

    parser.py
    â””â”€â”€ haskell_parser/      (parent)
        â””â”€â”€ src/          (parent)
            â””â”€â”€ haskell-parser/  (parent)
                â””â”€â”€ python/       (parent)
                    â””â”€â”€ packages/ (parent)
                        â””â”€â”€ code/     (parent)
                            â””â”€â”€ grammars/
                                â””â”€â”€ haskell/
                                    â”œâ”€â”€ haskell1.0.grammar
                                    â”œâ”€â”€ haskell1.1.grammar
                                    â”œâ”€â”€ haskell1.4.grammar
                                    â”œâ”€â”€ haskell5.grammar
                                    â”œâ”€â”€ haskell7.grammar
                                    â”œâ”€â”€ haskell8.grammar
                                    â”œâ”€â”€ haskell10.grammar
                                    â”œâ”€â”€ haskell14.grammar
                                    â”œâ”€â”€ haskell17.grammar
                                    â””â”€â”€ haskell21.grammar   â† default
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from haskell_lexer import tokenize_haskell
from haskell_lexer.lexer import DEFAULT_VERSION, VALID_VERSIONS
from lang_parser import ASTNode, GrammarParser

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

_GRAMMAR_ROOT = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"


def _resolve_grammar_path(version: str | None) -> Path:
    """Return the Path to the correct .grammar file for the requested version.

    ``version=None`` (or ``""``) loads the default ``haskell2010.grammar``
    file â€” the latest Haskell grammar.  Named versions load the corresponding
    versioned file from ``grammars/haskell/``.

    Args:
        version: One of ``"1.0"``, ``"1.1"``, ``"1.4"``, ``"5"``, ``"7"``,
            ``"8"``, ``"10"``, ``"14"``, ``"17"``, ``"21"``, ``None``, or
            ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.grammar`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized Haskell version.
    """
    if not version:
        return _GRAMMAR_ROOT / "haskell" / f"haskell{DEFAULT_VERSION}.grammar"
    if version not in VALID_VERSIONS:
        raise ValueError(
            f"Unknown Haskell version {version!r}. "
            f"Valid versions: {sorted(VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "haskell" / f"haskell{version}.grammar"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_haskell_parser(
    source: str, version: str | None = None
) -> GrammarParser:
    """Create a ``GrammarParser`` configured for a specific Haskell version.

    Args:
        source: The Haskell source code to parse.
        version: Optional Haskell version string â€” ``"1.0"`` through
            ``"21"``.  When omitted (or ``None`` / ``""``), the default
            Haskell 21 grammar is used.

    Returns:
        A ``GrammarParser`` instance configured with the selected grammar
        rules.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        parser = create_haskell_parser('public class Hello { }')
        ast = parser.parse()

        parser = create_haskell_parser('var x = 1;', '10')
        ast = parser.parse()
    """
    tokens = tokenize_haskell(source, version)
    grammar_path = _resolve_grammar_path(version)
    grammar = parse_parser_grammar(grammar_path.read_text(encoding="utf-8"))
    return GrammarParser(tokens, grammar)


def parse_haskell(source: str, version: str | None = None) -> ASTNode:
    """Parse Haskell source code and return an AST.

    This is the main entry point for the Haskell parser. Pass in a string
    of Haskell source code, and get back an ``ASTNode`` representing the
    root ``program`` node.

    Args:
        source: The Haskell source code to parse.
        version: Optional Haskell version string â€” ``"1.0"`` through
            ``"21"``.  When omitted (or ``None`` / ``""``), the default
            Haskell 21 grammar is used.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (Haskell 21) grammar
        ast = parse_haskell('public class Hello { }')

        # Haskell 8 â€” lambdas, streams
        ast = parse_haskell('int x = 1;', '8')

        # Haskell 10 â€” var keyword
        ast = parse_haskell('var x = 1;', '10')
    """
    parser = create_haskell_parser(source, version)
    return parser.parse()

