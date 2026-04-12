"""C# Lexer — tokenizes C# source code using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python, JavaScript, or Java can tokenize C# —
or any other language — simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

C# has tokens that differ from Java and Python — like nullable operators
(``?.``, ``??``), lambda arrows (``=>``), verbatim string literals
(``@"..."``, ``$"..."`` interpolated strings), and modern constructs like
``record``, ``init``, ``required``, ``file`` access modifiers, and primary
constructors. The grammar-driven approach handles all of these without any
new tokenization code: they are declared in the ``.tokens`` file, and the
``GrammarLexer`` compiles them into regex patterns at runtime.

This is one of the great strengths of the grammar-driven architecture. You
do not need a new tokenizer for every language. You need only a new grammar
file. The engine stays the same.

Version Support
---------------

This module supports all twelve C# versions tracked by the repo. Pass the
``version`` argument to select a specific version's grammar:

- ``"1.0"``  — C# 1.0 (2002): the original release. Classes, interfaces,
  delegates, properties, events, generics (via object), foreach, using.
- ``"2.0"``  — C# 2.0 (2005): generics (real), nullable types (``int?``),
  iterators (``yield``), anonymous methods, partial classes.
- ``"3.0"``  — C# 3.0 (2007): LINQ, lambda expressions (``=>``) implicitly
  typed local variables (``var``), extension methods, auto-properties,
  anonymous types, object/collection initializers.
- ``"4.0"``  — C# 4.0 (2010): dynamic binding (``dynamic``), named and
  optional parameters, covariance/contravariance for generics.
- ``"5.0"``  — C# 5.0 (2012): async/await (``async``, ``await``),
  caller-info attributes.
- ``"6.0"``  — C# 6.0 (2015): null-conditional operator (``?.``),
  string interpolation (``$"..."``) , expression-bodied members (``=>``),
  ``nameof``, ``using static``, exception filters (``when``).
- ``"7.0"``  — C# 7.0 (2017): tuples, out variables, pattern matching
  (``is``, ``switch``), local functions, ref returns, digit separators
  (``1_000_000``), binary literals (``0b...``).
- ``"8.0"``  — C# 8.0 (2019): nullable reference types (``string?``),
  switch expressions, default interface methods, ranges (``..``), indices
  (``^1``), recursive patterns, async streams.
- ``"9.0"``  — C# 9.0 (2020): records (``record``), init-only setters
  (``init``), top-level statements, ``with`` expressions, pattern matching
  improvements (``not``, ``and``, ``or``).
- ``"10.0"`` — C# 10.0 (2021): record structs (``record struct``), global
  ``using``, file-scoped namespaces, interpolated string handlers,
  ``required`` members (preview), ``CallerArgumentExpression``.
- ``"11.0"`` — C# 11.0 (2022): required members (``required``), raw string
  literals (triple-quoted strings), generic math, list patterns (``[1, 2, ..]``),
  file-local types (``file``), ref fields.
- ``"12.0"`` — C# 12.0 (2023): primary constructors, collection expressions
  (``[1, 2, 3]``), inline arrays, optional lambda parameters, ``alias any
  type``, experimental interceptors.

When no ``version`` is given (or ``None`` / ``""``), the latest version
(C# 12.0) is used as the default.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_csharp_lexer(source, version)`` — creates a ``GrammarLexer``
  configured for the requested C# version.

- ``tokenize_csharp(source, version)`` — the all-in-one function. Pass in
  C# source code, get back a list of tokens.

Locating the Grammar Files
--------------------------

Grammar files live in ``code/grammars/csharp/`` at the repository root::

    tokenizer.py
    └── csharp_lexer/       (parent)
        └── src/            (parent)
            └── csharp-lexer/   (parent)
                └── python/         (parent)
                    └── packages/   (parent)
                        └── code/       (parent)
                            └── grammars/
                                └── csharp/
                                    ├── csharp1.0.tokens
                                    ├── csharp2.0.tokens
                                    ├── csharp3.0.tokens
                                    ├── csharp4.0.tokens
                                    ├── csharp5.0.tokens
                                    ├── csharp6.0.tokens
                                    ├── csharp7.0.tokens
                                    ├── csharp8.0.tokens
                                    ├── csharp9.0.tokens
                                    ├── csharp10.0.tokens
                                    ├── csharp11.0.tokens
                                    └── csharp12.0.tokens   ← default
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

# Walk six levels up from this file to reach the repo root, then descend
# into grammars/.  The path is:
#
#   tokenizer.py          (this file)
#   └── csharp_lexer/     .parent
#       └── src/          .parent
#           └── csharp-lexer/ .parent
#               └── python/   .parent
#                   └── packages/ .parent
#                       └── code/  .parent  ← repo root / code
#                           └── grammars/
#
_GRAMMAR_ROOT = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"

# The set of valid version strings.  Each maps to a file under
# code/grammars/csharp/csharp<version>.tokens
VALID_VERSIONS: frozenset[str] = frozenset({
    "1.0", "2.0", "3.0", "4.0", "5.0", "6.0",
    "7.0", "8.0", "9.0", "10.0", "11.0", "12.0",
})

DEFAULT_VERSION = "12.0"


def _resolve_tokens_path(version: str | None) -> Path:
    """Return the Path to the correct .tokens file for the requested version.

    ``version=None`` (or ``""``) loads the default ``csharp12.0.tokens``
    file — the latest C# grammar.  Named versions load the corresponding
    versioned file from ``grammars/csharp/``.

    Args:
        version: One of ``"1.0"`` through ``"12.0"``, ``None``, or ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.tokens`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized C# version.

    Example::

        path = _resolve_tokens_path(None)      # -> grammars/csharp/csharp12.0.tokens
        path = _resolve_tokens_path("8.0")     # -> grammars/csharp/csharp8.0.tokens
        path = _resolve_tokens_path("3.0")     # -> grammars/csharp/csharp3.0.tokens
    """
    if not version:
        return _GRAMMAR_ROOT / "csharp" / f"csharp{DEFAULT_VERSION}.tokens"
    if version not in VALID_VERSIONS:
        raise ValueError(
            f"Unknown C# version {version!r}. "
            f"Valid versions: {sorted(VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "csharp" / f"csharp{version}.tokens"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_csharp_lexer(
    source: str, version: str | None = None
) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for a specific C# version.

    This factory function reads the appropriate ``.tokens`` file, parses it
    into a ``TokenGrammar`` object, and returns a ``GrammarLexer`` ready
    to tokenize the given source.

    The caller can then call ``lexer.tokenize()`` to produce a list of
    ``Token`` objects, or pass the lexer to downstream pipeline stages.

    Args:
        source: The C# source code to tokenize.
        version: Optional C# version string — ``"1.0"`` through
            ``"12.0"``.  When omitted (or ``None`` / ``""``), the default
            C# 12.0 grammar is used.

    Returns:
        A ``GrammarLexer`` instance configured with the selected token
        definitions.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        lexer = create_csharp_lexer('public class Hello { }', '8.0')
        tokens = lexer.tokenize()

        # With async/await (C# 5.0+)
        lexer = create_csharp_lexer('async Task DoWork() { await Task.Delay(1000); }', '5.0')
        tokens = lexer.tokenize()
    """
    tokens_path = _resolve_tokens_path(version)
    grammar = parse_token_grammar(tokens_path.read_text(encoding="utf-8"))
    return GrammarLexer(source, grammar)


def tokenize_csharp(
    source: str, version: str | None = None
) -> list[Token]:
    """Tokenize C# source code and return a list of tokens.

    This is the main entry point for the C# lexer. Pass in a string of C#
    source code, and get back a flat list of ``Token`` objects. The list
    always ends with an ``EOF`` token.

    The tokenizer is grammar-driven: it reads the appropriate
    ``csharp{version}.tokens`` file at call time, which means no new
    tokenizer code is needed when a new version of C# is released — just
    update the ``.tokens`` file.

    Args:
        source: The C# source code to tokenize.
        version: Optional C# version string — ``"1.0"`` through
            ``"12.0"``.  When omitted (or ``None`` / ``""``), the default
            C# 12.0 grammar is used.

    Returns:
        A list of ``Token`` objects.  The last token is always ``EOF``.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (C# 12.0) grammar
        tokens = tokenize_csharp('public class Hello { }')

        # C# 3.0 — var, LINQ, lambda
        tokens = tokenize_csharp('var x = items.Where(i => i > 0).ToList();', '3.0')

        # C# 5.0 — async/await
        tokens = tokenize_csharp('async Task Run() { await Task.Delay(100); }', '5.0')

        # C# 9.0 — records, init
        tokens = tokenize_csharp('record Point(int X, int Y);', '9.0')

        # C# 12.0 — primary constructors, collection expressions
        tokens = tokenize_csharp('class Greeter(string name) { }', '12.0')
    """
    lexer = create_csharp_lexer(source, version)
    return lexer.tokenize()
