"""ECMAScript 5 (2009) Parser — parses ES5 JavaScript into ASTs.

ES5 adds the debugger statement and getter/setter property syntax in
object literals on top of the ES3 grammar.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from ecmascript_es5_lexer import tokenize_es5

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "ecmascript"
)
ES5_GRAMMAR_PATH = GRAMMAR_DIR / "es5.grammar"


def create_es5_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for ECMAScript 5 (2009)."""
    tokens = tokenize_es5(source)
    grammar = parse_parser_grammar(ES5_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_es5(source: str) -> ASTNode:
    """Parse ECMAScript 5 source code and return an AST."""
    parser = create_es5_parser(source)
    return parser.parse()
