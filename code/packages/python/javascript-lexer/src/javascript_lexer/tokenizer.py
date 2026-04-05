"""JavaScript Lexer — tokenizes JavaScript source code using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python can tokenize JavaScript — or any other
language — simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

JavaScript has tokens that Python does not — like ``===`` (strict equality),
``!==`` (strict inequality), ``=>`` (arrow), and delimiters like ``{}``,
``[]``, ``;``, and ``.``. The grammar-driven approach handles all of these
without any new tokenization code: they are declared in the ``.tokens`` file,
and the ``GrammarLexer`` compiles them into regex patterns at runtime.

Version Support
---------------

This module supports all ECMAScript versions tracked by the repo. Pass the
``version`` argument to select a specific version's grammar:

- ``"es1"``    — ECMAScript 1 (June 1997): the first standardised JavaScript.
- ``"es3"``    — ECMAScript 3 (December 1999): regex literals, try/catch,
  ``do-while``.  The baseline for IE6–IE8.
- ``"es5"``    — ECMAScript 5 (December 2009): strict mode, ``JSON``,
  ``Array.prototype.forEach``, getters/setters.
- ``"es2015"`` — ECMAScript 2015 (ES6, June 2015): ``let``/``const``, arrow
  functions, classes, modules, template literals, destructuring, generators.
- ``"es2016"`` — ECMAScript 2016 (ES7): ``**`` exponentiation,
  ``Array.prototype.includes``.
- ``"es2017"`` — ECMAScript 2017: ``async``/``await``, ``Object.entries``,
  shared memory.
- ``"es2018"`` — ECMAScript 2018: rest/spread properties, ``for-await-of``,
  ``Promise.finally``, regex named groups.
- ``"es2019"`` — ECMAScript 2019: ``Array.flat``, ``Object.fromEntries``,
  optional ``catch`` binding.
- ``"es2020"`` — ECMAScript 2020: ``BigInt``, ``??`` nullish coalescing,
  ``?.`` optional chaining, ``globalThis``, ``import()``.
- ``"es2021"`` — ECMAScript 2021: ``??=``, ``||=``, ``&&=`` logical
  assignment, ``Promise.any``, ``String.replaceAll``.
- ``"es2022"`` — ECMAScript 2022: top-level ``await``, class fields/methods,
  ``at()`` indexing, ``Object.hasOwn``.
- ``"es2023"`` — ECMAScript 2023: ``Array.findLast``, ``#!`` hashbang
  comments, immutable ``Array.toReversed/toSorted/with``.
- ``"es2024"`` — ECMAScript 2024: ``Promise.withResolvers``,
  ``Object.groupBy``, ``ArrayBuffer.resize``.
- ``"es2025"`` — ECMAScript 2025: ``using``/``await using`` (explicit resource
  management), ``import defer``, ``RegExp.escape``, iterator helpers.

When no ``version`` is given, the generic ``javascript.tokens`` grammar is
used (equivalent to the latest stable feature set).

What This Module Provides
-------------------------

Two convenience functions:

- ``create_javascript_lexer(source, version)`` — creates a ``GrammarLexer``
  configured for the requested ECMAScript version.

- ``tokenize_javascript(source, version)`` — the all-in-one function. Pass in
  JavaScript source code, get back a list of tokens.

Locating the Grammar Files
--------------------------

Grammar files live in ``code/grammars/`` at the repository root::

    tokenizer.py
    └── javascript_lexer/   (parent)
        └── src/            (parent)
            └── javascript-lexer/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                ├── javascript.tokens          ← default (no version)
                                └── ecmascript/
                                    ├── es1.tokens
                                    ├── es3.tokens
                                    ├── es5.tokens
                                    ├── es2015.tokens
                                    ├── ...
                                    └── es2025.tokens
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
# code/grammars/ecmascript/<version>.tokens
_VALID_VERSIONS = frozenset({
    "es1", "es3", "es5",
    "es2015", "es2016", "es2017", "es2018", "es2019",
    "es2020", "es2021", "es2022", "es2023", "es2024", "es2025",
})


def _resolve_tokens_path(version: str | None) -> Path:
    """Return the Path to the correct .tokens file for the requested version.

    ``version=None`` (or ``""```) loads the generic ``javascript.tokens``
    file — the all-features grammar used as the default.  Named versions
    load the corresponding versioned file from ``grammars/ecmascript/``.

    Args:
        version: One of ``"es1"``, ``"es3"``, ``"es5"``, ``"es2015"`` …
            ``"es2025"``, ``None``, or ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.tokens`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized ECMAScript version.
    """
    if not version:
        return _GRAMMAR_ROOT / "javascript.tokens"
    if version not in _VALID_VERSIONS:
        raise ValueError(
            f"Unknown ECMAScript version {version!r}. "
            f"Valid versions: {sorted(_VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "ecmascript" / f"{version}.tokens"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_javascript_lexer(
    source: str, version: str | None = None
) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for a specific ECMAScript version.

    Args:
        source: The JavaScript source code to tokenize.
        version: Optional ECMAScript version string — ``"es1"`` through
            ``"es2025"``.  When omitted (or ``None`` / ``""``), the generic
            ``javascript.tokens`` grammar is used.

    Returns:
        A ``GrammarLexer`` instance configured with the selected token
        definitions.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        lexer = create_javascript_lexer('let x = 1 + 2;', 'es2015')
        tokens = lexer.tokenize()
    """
    tokens_path = _resolve_tokens_path(version)
    grammar = parse_token_grammar(tokens_path.read_text(encoding="utf-8"))
    return GrammarLexer(source, grammar)


def tokenize_javascript(
    source: str, version: str | None = None
) -> list[Token]:
    """Tokenize JavaScript source code and return a list of tokens.

    This is the main entry point for the JavaScript lexer. Pass in a string
    of JavaScript source code, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    Args:
        source: The JavaScript source code to tokenize.
        version: Optional ECMAScript version string — ``"es1"`` through
            ``"es2025"``.  When omitted (or ``None`` / ``""``), the generic
            ``javascript.tokens`` grammar is used.

    Returns:
        A list of ``Token`` objects.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (generic) grammar
        tokens = tokenize_javascript('let x = 1 + 2;')

        # ECMAScript 5 — the IE9+ baseline
        tokens = tokenize_javascript('var x = 1;', 'es5')

        # ECMAScript 2015 — first class syntax (let, const, arrow functions)
        tokens = tokenize_javascript('const f = x => x * 2;', 'es2015')

        # ECMAScript 2025 — explicit resource management
        tokens = tokenize_javascript('using r = getResource();', 'es2025')
    """
    lexer = create_javascript_lexer(source, version)
    return lexer.tokenize()
