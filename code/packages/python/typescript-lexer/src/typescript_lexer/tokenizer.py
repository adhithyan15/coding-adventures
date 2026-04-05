"""TypeScript Lexer — tokenizes TypeScript source code using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python and JavaScript can tokenize TypeScript
— or any other language — simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

TypeScript has tokens that JavaScript does not — like ``interface``, ``type``,
``enum``, ``namespace``, and type-annotation keywords like ``number``,
``string``, ``boolean``. The grammar-driven approach handles all of these
without any new tokenization code: they are declared in the ``.tokens`` file,
and the ``GrammarLexer`` compiles them into regex patterns at runtime.

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

When no ``version`` is given, the generic ``typescript.tokens`` grammar is used
(equivalent to the latest stable feature set).

What This Module Provides
-------------------------

Two convenience functions:

- ``create_typescript_lexer(source, version)`` — creates a ``GrammarLexer``
  configured for the requested TypeScript version.

- ``tokenize_typescript(source, version)`` — the all-in-one function. Pass in
  TypeScript source code, get back a list of tokens.

Locating the Grammar Files
---------------------------

Grammar files live in ``code/grammars/`` at the repository root::

    tokenizer.py
    └── typescript_lexer/   (parent)
        └── src/            (parent)
            └── typescript-lexer/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                ├── typescript.tokens          ← default (no version)
                                └── typescript/
                                    ├── ts1.0.tokens
                                    ├── ts2.0.tokens
                                    ├── ts3.0.tokens
                                    ├── ts4.0.tokens
                                    ├── ts5.0.tokens
                                    └── ts5.8.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

_GRAMMAR_ROOT = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"

# The set of valid version strings.  Each string maps to a file under
# code/grammars/typescript/<version>.tokens
_VALID_VERSIONS = frozenset({"ts1.0", "ts2.0", "ts3.0", "ts4.0", "ts5.0", "ts5.8"})


def _resolve_tokens_path(version: str | None) -> Path:
    """Return the Path to the correct .tokens file for the requested version.

    ``version=None`` (or ``""```) loads the generic ``typescript.tokens``
    file — the all-features grammar used as the default.  Named versions
    load the corresponding versioned file from ``grammars/typescript/``.

    Args:
        version: One of ``"ts1.0"``, ``"ts2.0"``, ``"ts3.0"``, ``"ts4.0"``,
            ``"ts5.0"``, ``"ts5.8"``, ``None``, or ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.tokens`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized TypeScript version.
    """
    if not version:
        return _GRAMMAR_ROOT / "typescript.tokens"
    if version not in _VALID_VERSIONS:
        raise ValueError(
            f"Unknown TypeScript version {version!r}. "
            f"Valid versions: {sorted(_VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "typescript" / f"{version}.tokens"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_typescript_lexer(
    source: str, version: str | None = None
) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for a specific TypeScript version.

    Args:
        source: The TypeScript source code to tokenize.
        version: Optional TypeScript version string — ``"ts1.0"`` through
            ``"ts5.8"``.  When omitted (or ``None`` / ``""``), the generic
            ``typescript.tokens`` grammar is used.

    Returns:
        A ``GrammarLexer`` instance configured with the selected token
        definitions.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        lexer = create_typescript_lexer('let x: number = 1 + 2;', 'ts1.0')
        tokens = lexer.tokenize()
    """
    tokens_path = _resolve_tokens_path(version)
    grammar = parse_token_grammar(tokens_path.read_text(encoding="utf-8"))
    return GrammarLexer(source, grammar)


def tokenize_typescript(
    source: str, version: str | None = None
) -> list[Token]:
    """Tokenize TypeScript source code and return a list of tokens.

    This is the main entry point for the TypeScript lexer. Pass in a string
    of TypeScript source code, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    Args:
        source: The TypeScript source code to tokenize.
        version: Optional TypeScript version string — ``"ts1.0"`` through
            ``"ts5.8"``.  When omitted (or ``None`` / ``""``), the generic
            ``typescript.tokens`` grammar is used.

    Returns:
        A list of ``Token`` objects.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (generic) grammar
        tokens = tokenize_typescript('let x: number = 1 + 2;')

        # TypeScript 1.0 — first public release
        tokens = tokenize_typescript('interface Foo { x: number; }', 'ts1.0')

        # TypeScript 5.8 — ES2025 baseline, using/await using
        tokens = tokenize_typescript('using x = getResource();', 'ts5.8')
    """
    lexer = create_typescript_lexer(source, version)
    return lexer.tokenize()
