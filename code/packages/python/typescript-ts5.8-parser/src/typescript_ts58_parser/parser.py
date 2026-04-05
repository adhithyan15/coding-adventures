"""TypeScript 5.8 (2025) Parser — parses TypeScript 5.8 into ASTs.

TypeScript 5.8 targets the ES2025 baseline, which standardizes three major
features: TC39 decorators, import attributes (``with`` clause), and explicit
resource management (``using`` / ``await using``).

TS 5.8-specific additions:

- ``export type *`` re-exports — re-export all types from a module
- ``export type * as Namespace from "..."`` — re-export with namespace
- ``import type`` from computed module specifiers
- Conditional types improvements
- ``--erasableSyntaxOnly`` mode (syntax enforcement)
- ``ambient_module_declaration`` — ``module "specifier" { ... }``

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``ts5.8.grammar`` file, tokenizes the source with the TS 5.8 lexer, and
produces an ``ASTNode`` tree.

Grammar Highlights
-------------------

Beyond the full TS 5.0 grammar, TS 5.8 adds:

- ``using_declaration`` — ``using x = resource();``
- ``await_using_declaration`` — ``await using db = await connect();``
- ``import_attributes`` — ``with { type: "json" }`` on imports
- ``ambient_module_declaration`` — ambient module blocks
- ``export type *`` forms in ``export_declaration``
- HASHBANG at the start of ``program`` (optional)
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from typescript_ts58_lexer import tokenize_ts58

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS58_GRAMMAR_PATH = GRAMMAR_DIR / "ts5.8.grammar"


def create_ts58_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for TypeScript 5.8 (2025).

    Args:
        source: The TypeScript 5.8 source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.

    Example::

        parser = create_ts58_parser('using x = getResource();')
        ast = parser.parse()
    """
    tokens = tokenize_ts58(source)
    grammar = parse_parser_grammar(TS58_GRAMMAR_PATH.read_text(encoding="utf-8"))
    return GrammarParser(tokens, grammar)


def parse_ts58(source: str) -> ASTNode:
    """Parse TypeScript 5.8 source code and return an AST.

    Args:
        source: The TypeScript 5.8 source code to parse.

    Returns:
        An ``ASTNode`` rooted at ``program``.

    Example::

        ast = parse_ts58('using x = getResource();')
        assert ast.rule_name == "program"
    """
    parser = create_ts58_parser(source)
    return parser.parse()
