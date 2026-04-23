"""MACSYMA parser — grammar-driven wrapper.

Reads ``macsyma.grammar`` from the repo's ``code/grammars/macsyma/``
directory, tokenizes source via the MACSYMA lexer, and runs the
generic ``GrammarParser`` over the tokens.

The result is an ``ASTNode`` tree whose ``rule_name`` values correspond
directly to the nonterminals in ``macsyma.grammar`` — ``program``,
``statement``, ``expression``, ``assign``, ``additive``, ``power``,
``postfix``, ``atom``, etc. The tree is deliberately "flat" and
concrete: later passes in ``macsyma-compiler`` flatten the precedence
cascade into the uniform ``IRApply`` form.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from macsyma_lexer import tokenize_macsyma

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
MACSYMA_GRAMMAR_PATH = GRAMMAR_DIR / "macsyma" / "macsyma.grammar"


def create_macsyma_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for MACSYMA source.

    Tokenizes via ``macsyma_lexer.tokenize_macsyma``, reads
    ``macsyma.grammar``, and constructs a ``GrammarParser`` ready to
    produce an AST.

    Args:
        source: The MACSYMA source text.

    Returns:
        A ``GrammarParser``. Call ``.parse()`` to get the ``ASTNode``.
    """
    tokens = tokenize_macsyma(source)
    grammar = parse_parser_grammar(MACSYMA_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_macsyma(source: str) -> ASTNode:
    """Parse MACSYMA source and return the AST.

    This is the main entry point. The returned ``ASTNode`` has
    ``rule_name="program"`` at the root, with children that are
    ``statement`` subtrees.

    Args:
        source: The MACSYMA source text.

    Returns:
        An ``ASTNode`` with ``rule_name="program"``.

    Raises:
        FileNotFoundError: If the grammar file cannot be found.
        LexerError: If tokenization fails.
        GrammarParseError: If the source does not parse.

    Example::

        ast = parse_macsyma("x + 1;")
        assert ast.rule_name == "program"
    """
    return create_macsyma_parser(source).parse()
