"""MACSYMA parser — grammar-driven wrapper.

The MACSYMA parser grammar (``code/grammars/macsyma/macsyma.grammar``) is
pre-compiled into ``macsyma_parser._grammar`` — see that module's header
for regeneration instructions. This module tokenizes source via the
MACSYMA lexer and runs the generic ``GrammarParser`` over the tokens
using that compiled grammar.

The result is an ``ASTNode`` tree whose ``rule_name`` values correspond
directly to the nonterminals in ``macsyma.grammar`` — ``program``,
``statement``, ``expression``, ``assign``, ``additive``, ``power``,
``postfix``, ``atom``, etc. The tree is deliberately "flat" and
concrete: later passes in ``macsyma-compiler`` flatten the precedence
cascade into the uniform ``IRApply`` form.
"""

from __future__ import annotations

from lang_parser import ASTNode, GrammarParser
from macsyma_lexer import tokenize_macsyma

from macsyma_parser._grammar import PARSER_GRAMMAR


def create_macsyma_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for MACSYMA source.

    Tokenizes via ``macsyma_lexer.tokenize_macsyma`` and constructs a
    ``GrammarParser`` using the pre-compiled ``PARSER_GRAMMAR`` — no
    file I/O.

    Args:
        source: The MACSYMA source text.

    Returns:
        A ``GrammarParser``. Call ``.parse()`` to get the ``ASTNode``.
    """
    tokens = tokenize_macsyma(source)
    return GrammarParser(tokens, PARSER_GRAMMAR)


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
        LexerError: If tokenization fails.
        GrammarParseError: If the source does not parse.

    Example::

        ast = parse_macsyma("x + 1;")
        assert ast.rule_name == "program"
    """
    return create_macsyma_parser(source).parse()


def format_macsyma_syntax_error(source: str, error: BaseException) -> str:
    """Format parser/lexer failures as a MACSYMA-style syntax diagnostic."""
    line = getattr(getattr(error, "token", None), "line", None)
    column = getattr(getattr(error, "token", None), "column", None)
    message = _strip_parse_prefix(str(error))
    if isinstance(line, int) and isinstance(column, int) and line >= 1 and column >= 1:
        source_line = source.splitlines()[line - 1] if line <= len(source.splitlines()) else ""
        caret = " " * (column - 1) + "^"
        return f"Incorrect syntax at line {line}, column {column}: {message}\n{source_line}\n{caret}"
    return f"Incorrect syntax: {message}"


def _strip_parse_prefix(message: str) -> str:
    prefixes = ("Parse error: ",)
    for prefix in prefixes:
        if message.startswith(prefix):
            return message[len(prefix) :]
    if message.startswith("Parse error at "):
        _, _, rest = message.partition(": ")
        return rest or message
    return message
