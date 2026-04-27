"""ECMAScript 1 (1997) Parser — parses ES1 JavaScript into ASTs using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``es1.grammar`` file and uses the ES1 lexer to tokenize source code before
parsing.

The pipeline is:

1. Read ``es1.tokens`` -> ``GrammarLexer`` -> tokens (via ecmascript-es1-lexer)
2. Read ``es1.grammar`` -> ``GrammarParser`` -> AST

Locating the Grammar File
--------------------------

::

    parser.py → ecmascript_es1_parser/ → src/ → ecmascript-es1-parser/
    → python/ → packages/ → code/ → grammars/ecmascript/es1.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from ecmascript_es1_lexer import tokenize_es1

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "ecmascript"
)
ES1_GRAMMAR_PATH = GRAMMAR_DIR / "es1.grammar"


def create_es1_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for ECMAScript 1 (1997).

    Args:
        source: The ES1 JavaScript source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.

    Example::

        parser = create_es1_parser('var x = 1 + 2;')
        ast = parser.parse()
    """
    tokens = tokenize_es1(source)
    grammar = parse_parser_grammar(ES1_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_es1(source: str) -> ASTNode:
    """Parse ECMAScript 1 source code and return an AST.

    Args:
        source: The ES1 JavaScript source code to parse.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Example::

        ast = parse_es1('var x = 1 + 2;')
        # ASTNode(rule_name="program", children=[...])
    """
    parser = create_es1_parser(source)
    return parser.parse()
