"""Twig parser — thin wrapper around the generic ``GrammarParser``.

The Twig parser grammar is compiled into ``_grammar_parser.py`` by the
grammar-tools compiler from ``code/grammars/twig/twig.grammar``; this
module imports the pre-built ``PARSER_GRAMMAR`` constant and feeds it the
token stream produced by :mod:`twig.lexer`.  The result is a generic
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

To regenerate after editing ``code/grammars/twig/twig.grammar``::

    grammar-tools compile-grammar code/grammars/twig/twig.grammar \\
        > code/packages/python/twig/src/twig/_grammar_parser.py
"""

from __future__ import annotations

from lang_parser import ASTNode, GrammarParser

from twig._grammar_parser import PARSER_GRAMMAR
from twig.lexer import tokenize_twig


def create_twig_parser(source: str) -> GrammarParser:
    """Build a ``GrammarParser`` ready to parse Twig source.

    Combines the Twig token stream with the pre-compiled
    ``PARSER_GRAMMAR`` (from ``twig._grammar_parser``).  Call
    ``.parse()`` to get the AST root.
    """
    tokens = tokenize_twig(source)
    return GrammarParser(tokens, PARSER_GRAMMAR)


def parse_twig(source: str) -> ASTNode:
    """Parse Twig source into a generic ``ASTNode`` tree.

    Raises whatever the underlying ``GrammarParser`` raises on
    malformed input (typically a parse error mentioning the token
    that triggered the failure).  An empty source returns a
    ``program`` node with no children.
    """
    return create_twig_parser(source).parse()
