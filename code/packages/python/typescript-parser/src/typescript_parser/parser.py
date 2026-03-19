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

Locating the Grammar File
--------------------------

The ``typescript.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    parser.py
    └── typescript_parser/  (parent)
        └── src/            (parent)
            └── typescript-parser/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── typescript.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from typescript_lexer import tokenize_typescript

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
TS_GRAMMAR_PATH = GRAMMAR_DIR / "typescript.grammar"


def create_typescript_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for TypeScript source code.

    Args:
        source: The TypeScript source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.

    Example::

        parser = create_typescript_parser('let x = 1 + 2;')
        ast = parser.parse()
    """
    tokens = tokenize_typescript(source)
    grammar = parse_parser_grammar(TS_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_typescript(source: str) -> ASTNode:
    """Parse TypeScript source code and return an AST.

    This is the main entry point for the TypeScript parser.

    Args:
        source: The TypeScript source code to parse.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Example::

        ast = parse_typescript('let x = 1 + 2;')
        # ASTNode(rule_name="program", children=[
        #     ASTNode(rule_name="statement", children=[
        #         ASTNode(rule_name="var_declaration", children=[...])
        #     ])
        # ])
    """
    parser = create_typescript_parser(source)
    return parser.parse()
