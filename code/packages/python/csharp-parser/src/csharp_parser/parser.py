"""C# Parser — parses C# source code into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the C# lexer: the *same* parser engine
that handles Python, JavaScript, or Java can handle C# — just swap the
``.grammar`` file.

C# has constructs that are unique among mainstream languages:

- **Properties** — ``public int Age { get; set; }`` — first-class getters
  and setters that look like fields but behave like methods.
- **Delegates and events** — a built-in callback mechanism:
  ``public event EventHandler Clicked;``
- **Nullable value types** — ``int?`` means an ``int`` that can be null,
  backed by ``System.Nullable<int>``.
- **LINQ query syntax** — SQL-like queries embedded in the language:
  ``from x in items where x > 0 select x``
- **Records** — immutable data types with value semantics:
  ``record Point(int X, int Y);``
- **Primary constructors** — constructor parameters directly in the class
  head: ``class Greeter(string name) { }``

The grammar-driven approach handles all of these through grammar rules
rather than hard-coded parser logic.

The pipeline is:

1. Read ``csharp{version}.tokens`` -> build ``TokenGrammar`` ->
   ``GrammarLexer`` -> tokens
2. Read ``csharp{version}.grammar`` -> build ``ParserGrammar`` ->
   ``GrammarParser`` -> AST

Version Support
---------------

This module supports all twelve C# versions tracked by the repo. Pass the
``version`` argument to select a specific version's grammar:

- ``"1.0"``  — C# 1.0 (2002)
- ``"2.0"``  — C# 2.0 (2005)
- ``"3.0"``  — C# 3.0 (2007)
- ``"4.0"``  — C# 4.0 (2010)
- ``"5.0"``  — C# 5.0 (2012)
- ``"6.0"``  — C# 6.0 (2015)
- ``"7.0"``  — C# 7.0 (2017)
- ``"8.0"``  — C# 8.0 (2019)
- ``"9.0"``  — C# 9.0 (2020)
- ``"10.0"`` — C# 10.0 (2021)
- ``"11.0"`` — C# 11.0 (2022)
- ``"12.0"`` — C# 12.0 (2023)

When no ``version`` is given, the default C# 12.0 grammar is used
(the latest version).

Locating the Grammar Files
--------------------------

Grammar files live in ``code/grammars/csharp/`` at the repository root::

    parser.py
    └── csharp_parser/      (parent)
        └── src/            (parent)
            └── csharp-parser/  (parent)
                └── python/         (parent)
                    └── packages/   (parent)
                        └── code/       (parent)
                            └── grammars/
                                └── csharp/
                                    ├── csharp1.0.grammar
                                    ├── csharp2.0.grammar
                                    ├── csharp3.0.grammar
                                    ├── csharp4.0.grammar
                                    ├── csharp5.0.grammar
                                    ├── csharp6.0.grammar
                                    ├── csharp7.0.grammar
                                    ├── csharp8.0.grammar
                                    ├── csharp9.0.grammar
                                    ├── csharp10.0.grammar
                                    ├── csharp11.0.grammar
                                    └── csharp12.0.grammar   ← default
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from csharp_lexer import tokenize_csharp
from csharp_lexer.tokenizer import DEFAULT_VERSION, VALID_VERSIONS
from lang_parser import ASTNode, GrammarParser

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

# Walk six levels up from this file to reach the repo root, then descend
# into grammars/.  The path is:
#
#   parser.py              (this file)
#   └── csharp_parser/     .parent
#       └── src/           .parent
#           └── csharp-parser/ .parent
#               └── python/    .parent
#                   └── packages/ .parent
#                       └── code/  .parent  ← repo root / code
#                           └── grammars/
#
_GRAMMAR_ROOT = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"


def _resolve_grammar_path(version: str | None) -> Path:
    """Return the Path to the correct .grammar file for the requested version.

    ``version=None`` (or ``""``) loads the default ``csharp12.0.grammar``
    file — the latest C# grammar.  Named versions load the corresponding
    versioned file from ``grammars/csharp/``.

    Args:
        version: One of ``"1.0"`` through ``"12.0"``, ``None``, or ``""``.

    Returns:
        Absolute ``Path`` to the selected ``.grammar`` file.

    Raises:
        ValueError: If ``version`` is a non-empty string that is not a
            recognized C# version.

    Example::

        path = _resolve_grammar_path(None)     # -> grammars/csharp/csharp12.0.grammar
        path = _resolve_grammar_path("8.0")    # -> grammars/csharp/csharp8.0.grammar
        path = _resolve_grammar_path("3.0")    # -> grammars/csharp/csharp3.0.grammar
    """
    if not version:
        return _GRAMMAR_ROOT / "csharp" / f"csharp{DEFAULT_VERSION}.grammar"
    if version not in VALID_VERSIONS:
        raise ValueError(
            f"Unknown C# version {version!r}. "
            f"Valid versions: {sorted(VALID_VERSIONS)}"
        )
    return _GRAMMAR_ROOT / "csharp" / f"csharp{version}.grammar"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_csharp_parser(
    source: str, version: str | None = None
) -> GrammarParser:
    """Create a ``GrammarParser`` configured for a specific C# version.

    This factory function:
    1. Tokenizes the source using ``csharp_lexer.tokenize_csharp()``.
    2. Reads the appropriate ``csharp{version}.grammar`` file.
    3. Parses that file into a ``ParserGrammar`` using ``grammar_tools``.
    4. Returns a ``GrammarParser`` ready to call ``.parse()`` on.

    The caller can then call ``parser.parse()`` to produce an ``ASTNode``
    tree rooted at the ``program`` rule.

    Args:
        source: The C# source code to parse.
        version: Optional C# version string — ``"1.0"`` through
            ``"12.0"``.  When omitted (or ``None`` / ``""``), the default
            C# 12.0 grammar is used.

    Returns:
        A ``GrammarParser`` instance configured with the selected grammar
        rules.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        parser = create_csharp_parser('public class Hello { }')
        ast = parser.parse()

        # C# 9.0 — records
        parser = create_csharp_parser('record Point(int X, int Y);', '9.0')
        ast = parser.parse()
    """
    tokens = tokenize_csharp(source, version)
    grammar_path = _resolve_grammar_path(version)
    grammar = parse_parser_grammar(grammar_path.read_text(encoding="utf-8"))
    return GrammarParser(tokens, grammar)


def parse_csharp(source: str, version: str | None = None) -> ASTNode:
    """Parse C# source code and return an AST.

    This is the main entry point for the C# parser. Pass in a string of C#
    source code, and get back an ``ASTNode`` representing the root
    ``program`` node of the parse tree.

    The pipeline is fully grammar-driven: the same generic ``GrammarParser``
    that processes Java or JavaScript processes C# — only the grammar file
    differs.

    Args:
        source: The C# source code to parse.
        version: Optional C# version string — ``"1.0"`` through
            ``"12.0"``.  When omitted (or ``None`` / ``""``), the default
            C# 12.0 grammar is used.

    Returns:
        An ``ASTNode`` representing the root ``program`` node of the parse
        tree.

    Raises:
        ValueError: If ``version`` is not a recognized version string.

    Example::

        # Default (C# 12.0) grammar
        ast = parse_csharp('public class Hello { }')
        print(ast.rule_name)  # "program"

        # C# 8.0 — nullable refs, switch expressions
        ast = parse_csharp('int x = 1;', '8.0')

        # C# 9.0 — records
        ast = parse_csharp('record Point(int X, int Y);', '9.0')

        # C# 5.0 — async/await
        ast = parse_csharp('async Task Main() { }', '5.0')
    """
    parser = create_csharp_parser(source, version)
    return parser.parse()
