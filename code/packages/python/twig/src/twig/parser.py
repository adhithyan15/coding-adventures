"""Twig parser — thin wrapper around the generic ``GrammarParser``.

Loads ``code/grammars/twig.grammar`` and feeds it the token stream
produced by :mod:`twig.lexer`.  The result is a generic
:class:`lang_parser.ASTNode` tree whose ``rule_name`` fields match
the production names from the grammar (``program``, ``define``,
``if_form``, ``let_form``, ``begin_form``, ``lambda_form``,
``quote_form``, ``apply``, ``atom``, ``binding``, …).

Downstream walkers (``twig.free_vars``, ``twig.compiler``) dispatch
on ``rule_name`` to interpret each subtree.  This matches how
``brainfuck-iir-compiler`` consumes the Brainfuck AST: the lexer
and parser are language-agnostic infrastructure, and the language
package supplies thin lex/parse wrappers plus a typed walker that
turns the generic AST into the language's semantic actions.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser

from twig.lexer import tokenize_twig

# Grammar file location — same walk-up as ``lexer.py``.
GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
TWIG_GRAMMAR_PATH = GRAMMAR_DIR / "twig.grammar"


def create_twig_parser(source: str) -> GrammarParser:
    """Build a ``GrammarParser`` ready to parse Twig source.

    Combines the Twig token stream with the rules from
    ``twig.grammar``.  Call ``.parse()`` to get the AST root.
    """
    tokens = tokenize_twig(source)
    grammar = parse_parser_grammar(TWIG_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_twig(source: str) -> ASTNode:
    """Parse Twig source into a generic ``ASTNode`` tree.

    Raises whatever the underlying ``GrammarParser`` raises on
    malformed input (typically a parse error mentioning the token
    that triggered the failure).  An empty source returns a
    ``program`` node with no children.
    """
    return create_twig_parser(source).parse()
