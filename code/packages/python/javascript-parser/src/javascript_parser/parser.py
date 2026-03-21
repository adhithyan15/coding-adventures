"""JavaScript Parser — parses JavaScript source code into ASTs using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the JavaScript lexer: the *same* parser
engine that handles Python can handle JavaScript — just swap the ``.grammar`` file.

The JavaScript grammar has a ``var_declaration`` rule that Python and Ruby
do not: ``KEYWORD NAME EQUALS expression SEMICOLON``. This handles
``let x = 1;``, ``const y = 2;``, and ``var z = 3;``.

The pipeline is:

1. Read ``javascript.tokens`` -> build ``TokenGrammar`` -> ``GrammarLexer`` -> tokens
2. Read ``javascript.grammar`` -> build ``ParserGrammar`` -> ``GrammarParser`` -> AST

Locating the Grammar File
--------------------------

The ``javascript.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    parser.py
    └── javascript_parser/  (parent)
        └── src/            (parent)
            └── javascript-parser/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── javascript.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from javascript_lexer import tokenize_javascript

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
JS_GRAMMAR_PATH = GRAMMAR_DIR / "javascript.grammar"


def create_javascript_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for JavaScript source code.

    Args:
        source: The JavaScript source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.

    Example::

        parser = create_javascript_parser('let x = 1 + 2;')
        ast = parser.parse()
    """
    tokens = tokenize_javascript(source)
    grammar = parse_parser_grammar(JS_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_javascript(source: str) -> ASTNode:
    """Parse JavaScript source code and return an AST.

    This is the main entry point for the JavaScript parser.

    Args:
        source: The JavaScript source code to parse.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Example::

        ast = parse_javascript('let x = 1 + 2;')
        # ASTNode(rule_name="program", children=[
        #     ASTNode(rule_name="statement", children=[
        #         ASTNode(rule_name="var_declaration", children=[...])
        #     ])
        # ])
    """
    parser = create_javascript_parser(source)
    return parser.parse()
