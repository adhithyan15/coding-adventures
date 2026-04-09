"""JavaScript Parser — parses JavaScript source code into ASTs using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the JavaScript lexer: the *same* parser
engine that handles Python can handle JavaScript — just swap the ``.grammar``
file.

The JavaScript grammar has a ``var_declaration`` rule that Python and Ruby
do not: ``KEYWORD NAME EQUALS expression SEMICOLON``. This handles
``let x = 1;``, ``const y = 2;``, and ``var z = 3;``.

The pipeline is:

1. Read ``javascript.tokens`` -> build ``TokenGrammar`` -> ``GrammarLexer`` -> tokens
2. Read ``javascript.grammar`` -> build ``ParserGrammar`` -> ``GrammarParser`` -> AST

Version Support
---------------

This module supports all ECMAScript versions tracked by the repo. Pass the
``version`` argument to select a specific version's grammar:

- ``"es1"``    — ECMAScript 1 (June 1997)
- ``"es3"``    — ECMAScript 3 (December 1999)
- ``"es5"``    — ECMAScript 5 (December 2009)
- ``"es2015"`` — ECMAScript 2015 (ES6)
- ``"es2016"`` through ``"es2025"`` — annual ES releases.

When no ``version`` is given, the generic ``javascript.grammar`` is used
(equivalent to the latest stable feature set).

Locating the Grammar Files
--------------------------

Grammar files live in ``code/grammars/`` at the repository root::

    parser.py
    └── javascript_parser/  (parent)
        └── src/            (parent)
            └── javascript-parser/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                ├── javascript.grammar          ← default (no version)
                                └── ecmascript/
                                    ├── es1.grammar
                                    ├── es3.grammar
                                    ├── es5.grammar
                                    ├── es2015.grammar
                                    ├── ...
                                    └── es2025.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from javascript_lexer import tokenize_javascript

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

_GRAMMAR_ROOT = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"

# The set of valid version strings.  Each maps to a file under
# code/grammars/ecmascript/<version>.grammar
_VALID_VERSIONS = frozenset({
    "es1", "es3", "es5",
    "es2015", "es2016", "es2017", "es2018", "es2019",
    "es2020", "es2021", "es2022", "es2023", "es2024", "es2025",
})


def _resolve_grammar_path(version: str | None) -> Path:
    """Return the Path to the correct .grammar file for the requested version.

    ``version=None`` (or ``""```) loads the generic ``javascript.grammar``
    file — the all-features grammar used as the default.  Named versions
    load the corresponding versioned file from ``grammars/ecmascript/``.

    Args:
        version: One of ``"es1"``, ``"es3"``, ``"es5"``, ``"es2015"`` …
            ``"es2025"``, ``None``, or ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.grammar`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized ECMAScript version.
    """
    if not version:
        return _GRAMMAR_ROOT / "javascript.grammar"
    if version not in _VALID_VERSIONS:
        raise ValueError(
            f"Unknown ECMAScript version {version!r}. "
            f"Valid versions: {sorted(_VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "ecmascript" / f"{version}.grammar"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_javascript_parser(
    source: str, version: str | None = None
) -> GrammarParser:
    """Create a ``GrammarParser`` configured for a specific ECMAScript version.

    Args:
        source: The JavaScript source code to parse.
        version: Optional ECMAScript version string — ``"es1"`` through
            ``"es2025"``.  When omitted (or ``None`` / ``""``), the generic
            ``javascript.grammar`` grammar is used.

    Returns:
        A ``GrammarParser`` instance configured with the selected grammar
        rules.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        parser = create_javascript_parser('let x = 1 + 2;')
        ast = parser.parse()

        parser = create_javascript_parser('var x = 1;', 'es5')
        ast = parser.parse()
    """
    tokens = tokenize_javascript(source, version)
    grammar_path = _resolve_grammar_path(version)
    grammar = parse_parser_grammar(grammar_path.read_text(encoding="utf-8"))
    return GrammarParser(tokens, grammar)


def parse_javascript(source: str, version: str | None = None) -> ASTNode:
    """Parse JavaScript source code and return an AST.

    This is the main entry point for the JavaScript parser. Pass in a string
    of JavaScript source code, and get back an ``ASTNode`` representing the
    root ``program`` node.

    Args:
        source: The JavaScript source code to parse.
        version: Optional ECMAScript version string — ``"es1"`` through
            ``"es2025"``.  When omitted (or ``None`` / ``""``), the generic
            ``javascript.grammar`` grammar is used.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (generic) grammar
        ast = parse_javascript('let x = 1 + 2;')

        # ECMAScript 5 — the IE9+ baseline
        ast = parse_javascript('var x = 1;', 'es5')

        # ECMAScript 2015 — let/const, arrow functions
        ast = parse_javascript('let x = 1;', 'es2015')
    """
    parser = create_javascript_parser(source, version)
    return parser.parse()
