"""ECMAScript 3 (1999) Parser — parses ES3 JavaScript into ASTs.

ES3 adds try/catch/finally/throw statements, strict equality (=== !==),
instanceof, and regex literals to the ES1 grammar.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from ecmascript_es3_lexer import tokenize_es3

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "ecmascript"
)
ES3_GRAMMAR_PATH = GRAMMAR_DIR / "es3.grammar"


def create_es3_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for ECMAScript 3 (1999)."""
    tokens = tokenize_es3(source)
    grammar = parse_parser_grammar(ES3_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_es3(source: str) -> ASTNode:
    """Parse ECMAScript 3 source code and return an AST."""
    parser = create_es3_parser(source)
    return parser.parse()
