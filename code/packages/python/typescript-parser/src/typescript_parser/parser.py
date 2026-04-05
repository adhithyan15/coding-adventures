"""TypeScript Parser — parses TypeScript source code into ASTs using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the TypeScript lexer: the *same* parser
engine that handles Python and JavaScript can handle TypeScript — just swap
the ``.grammar`` file.

The TypeScript grammar extends the JavaScript grammar with type annotations,
interface declarations, and other TypeScript-specific constructs. The
``var_declaration`` rule still handles ``let x = 1;``, ``const y = 2;``,
and ``var z = 3;``.

The pipeline is:

1. Read ``typescript.tokens`` -> build ``TokenGrammar`` -> ``GrammarLexer`` -> tokens
2. Read ``typescript.grammar`` -> build ``ParserGrammar`` -> ``GrammarParser`` -> AST

Version Support
---------------

This module supports all six TypeScript versions that the repo tracks. Pass
the ``version`` argument to select a specific version's grammar:

- ``"ts1.0"`` — TypeScript 1.0 (April 2014): interfaces, classes, enums,
  generics, namespaces, ambient declarations.
- ``"ts2.0"`` — TypeScript 2.0 (September 2016): union/intersection types,
  control flow analysis, ``never``, ``readonly``, tagged template literals.
- ``"ts3.0"`` — TypeScript 3.0 (July 2018): rest parameter types, tuple
  type improvements, ``unknown`` type.
- ``"ts4.0"`` — TypeScript 4.0 (August 2020): variadic tuple types, labeled
  tuple elements, short-circuit assignment operators (``&&=``, ``||=``, ``??=``).
- ``"ts5.0"`` — TypeScript 5.0 (March 2023): ``const`` type parameters,
  decorators (TC39 Stage 3), tsconfig extends arrays.
- ``"ts5.8"`` — TypeScript 5.8 (February 2025): ES2025 baseline, ``using`` /
  ``await using``, import attributes, ``export type *``.

When no ``version`` is given, the generic ``typescript.grammar`` grammar is
used (equivalent to the latest stable feature set).

Locating the Grammar Files
--------------------------

Grammar files live in ``code/grammars/`` at the repository root::

    parser.py
    └── typescript_parser/  (parent)
        └── src/            (parent)
            └── typescript-parser/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                ├── typescript.grammar          ← default (no version)
                                └── typescript/
                                    ├── ts1.0.grammar
                                    ├── ts2.0.grammar
                                    ├── ts3.0.grammar
                                    ├── ts4.0.grammar
                                    ├── ts5.0.grammar
                                    └── ts5.8.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from typescript_lexer import tokenize_typescript

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

_GRAMMAR_ROOT = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"

# The set of valid version strings.  Each string maps to a file under
# code/grammars/typescript/<version>.grammar
_VALID_VERSIONS = frozenset({"ts1.0", "ts2.0", "ts3.0", "ts4.0", "ts5.0", "ts5.8"})


def _resolve_grammar_path(version: str | None) -> Path:
    """Return the Path to the correct .grammar file for the requested version.

    ``version=None`` (or ``""```) loads the generic ``typescript.grammar``
    file — the all-features grammar used as the default.  Named versions
    load the corresponding versioned file from ``grammars/typescript/``.

    Args:
        version: One of ``"ts1.0"``, ``"ts2.0"``, ``"ts3.0"``, ``"ts4.0"``,
            ``"ts5.0"``, ``"ts5.8"``, ``None``, or ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.grammar`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized TypeScript version.
    """
    if not version:
        return _GRAMMAR_ROOT / "typescript.grammar"
    if version not in _VALID_VERSIONS:
        raise ValueError(
            f"Unknown TypeScript version {version!r}. "
            f"Valid versions: {sorted(_VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "typescript" / f"{version}.grammar"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_typescript_parser(
    source: str, version: str | None = None
) -> GrammarParser:
    """Create a ``GrammarParser`` configured for a specific TypeScript version.

    Args:
        source: The TypeScript source code to parse.
        version: Optional TypeScript version string — ``"ts1.0"`` through
            ``"ts5.8"``.  When omitted (or ``None`` / ``""``), the generic
            ``typescript.grammar`` grammar is used.

    Returns:
        A ``GrammarParser`` instance configured with the selected grammar
        rules.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        parser = create_typescript_parser('let x = 1 + 2;')
        ast = parser.parse()

        # TypeScript 1.0 — interface syntax
        parser = create_typescript_parser('interface Foo { x: number; }', 'ts1.0')
        ast = parser.parse()
    """
    tokens = tokenize_typescript(source, version)
    grammar_path = _resolve_grammar_path(version)
    grammar = parse_parser_grammar(grammar_path.read_text(encoding="utf-8"))
    return GrammarParser(tokens, grammar)


def parse_typescript(source: str, version: str | None = None) -> ASTNode:
    """Parse TypeScript source code and return an AST.

    This is the main entry point for the TypeScript parser. Pass in a string
    of TypeScript source code, and get back an ``ASTNode`` representing the
    root ``program`` node.

    Args:
        source: The TypeScript source code to parse.
        version: Optional TypeScript version string — ``"ts1.0"`` through
            ``"ts5.8"``.  When omitted (or ``None`` / ``""``), the generic
            ``typescript.grammar`` grammar is used.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (generic) grammar
        ast = parse_typescript('let x = 1 + 2;')

        # TypeScript 1.0 — first public release
        ast = parse_typescript('var x: number = 1;', 'ts1.0')

        # TypeScript 5.8 — ES2025 baseline
        ast = parse_typescript('let x = 1;', 'ts5.8')
    """
    parser = create_typescript_parser(source, version)
    return parser.parse()
