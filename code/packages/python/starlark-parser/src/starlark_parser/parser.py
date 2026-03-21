"""Starlark Parser — parses Starlark source code into ASTs using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the Starlark lexer: the *same* parser
engine that handles Python can handle Starlark — just swap the ``.grammar``
file.

How the Grammar-Driven Parser Works (Brief Recap)
--------------------------------------------------

The ``GrammarParser`` interprets EBNF grammar rules at runtime. For each
rule, it tries to match the token stream against the rule's body using
a recursive descent approach with backtracking:

- **Sequences** (``A B C``) must match all elements in order.
- **Alternations** (``A | B``) try each choice; first match wins.
- **Repetitions** (``{ A }``) match zero or more times.
- **Optionals** (``[ A ]``) match zero or one time.
- **Token references** (``INT``, ``NAME``) match tokens by type.
- **Literals** (``"def"``) match tokens by exact text value.
- **Rule references** (``expression``) recursively parse another rule.

The parser produces a tree of generic ``ASTNode`` objects, where each
node records which grammar rule produced it and what children it matched.

The Starlark Grammar
---------------------

The ``starlark.grammar`` file defines a complete Starlark parser covering:

- **Top-level structure**: A file is a sequence of statements.
- **Simple statements**: Assignment, return, break, continue, pass, load.
- **Compound statements**: if/elif/else, for, def — all with indented suites.
- **Expressions**: Full operator precedence from lambda (lowest) to
  primary expressions (highest), with 15 precedence levels.
- **Comprehensions**: List and dict comprehensions with for/if clauses.
- **Function calls**: Positional, keyword, *args, and **kwargs arguments.

This is significantly more complex than the Ruby grammar, reflecting
Starlark's richer expression syntax (borrowed from Python).

What This Module Provides
-------------------------

Two convenience functions:

- ``create_starlark_parser(source)`` — tokenizes the source with
  ``starlark_lexer`` and creates a ``GrammarParser`` configured with the
  Starlark grammar. Use this when you want to control the parse process.

- ``parse_starlark(source)`` — the all-in-one function. Pass in Starlark
  source code, get back an AST. This is the function most callers want.

Locating the Grammar File
--------------------------

The ``starlark.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path, similar to how
the Starlark lexer locates ``starlark.tokens``.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from starlark_lexer import tokenize_starlark

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# We navigate from this file's location up to the repository root's
# grammars/ directory. The path traversal is:
#   src/starlark_parser/parser.py -> src/starlark_parser -> src ->
#   starlark-parser -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
STARLARK_GRAMMAR_PATH = GRAMMAR_DIR / "starlark.grammar"


def create_starlark_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Starlark source code.

    This function:

    1. Tokenizes the source code using the Starlark lexer.
    2. Reads and parses the ``starlark.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Use this when you want access to the parser object itself — for
    example, to inspect its internal state or integrate with a custom
    pipeline. For most use cases, ``parse_starlark()`` is simpler.

    Args:
        source: The Starlark source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters or
            reserved keywords.

    Example::

        parser = create_starlark_parser('x = 1 + 2\\n')
        ast = parser.parse()
    """
    tokens = tokenize_starlark(source)
    grammar = parse_parser_grammar(STARLARK_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_starlark(source: str) -> ASTNode:
    """Parse Starlark source code and return an AST.

    This is the main entry point for the Starlark parser. Pass in a string
    of Starlark source code, and get back an ``ASTNode`` representing the
    complete parse tree.

    The returned AST has the following structure:

    - The root node has ``rule_name="file"`` and its children are
      the statements in the file (plus any NEWLINE tokens between them).
    - Each statement is dispatched to either ``simple_stmt`` or
      ``compound_stmt``, and then to specific statement rules like
      ``assign_stmt``, ``if_stmt``, ``for_stmt``, or ``def_stmt``.
    - Expressions follow the standard precedence hierarchy from
      ``expression`` down through ``or_expr``, ``and_expr``, ``comparison``,
      ``arith``, ``term``, ``factor``, ``power``, ``primary``, to ``atom``.

    **Important**: Starlark source code should typically end with a newline
    character (``\\n``). The lexer's indentation mode expects logical lines
    to be terminated by newlines, and omitting the trailing newline may
    cause unexpected behavior.

    Args:
        source: The Starlark source code to parse.

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is ``"file"``.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters or
            reserved keywords.
        GrammarParseError: If the source has syntax errors according
            to the Starlark grammar.

    Example::

        ast = parse_starlark('x = 1 + 2\\n')
        # ASTNode(rule_name="file", children=[
        #     ASTNode(rule_name="statement", children=[
        #         ASTNode(rule_name="simple_stmt", children=[
        #             ASTNode(rule_name="assign_stmt", children=[...])
        #         ])
        #     ])
        # ])
    """
    parser = create_starlark_parser(source)
    return parser.parse()
