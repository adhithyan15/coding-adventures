"""TypeScript 3.0 (2018) Parser — parses TypeScript 3.0 source code into ASTs.

TypeScript 3.0 introduced the ``unknown`` top type and rest/spread in tuple
types on an ES2018 baseline. This parser produces ``ASTNode`` trees from
TypeScript 3.0 source.

Architecture
------------

This module is a thin wrapper around the generic ``GrammarParser``. It:

1. Tokenizes the source using ``tokenize_ts30`` from the sibling lexer package.
2. Loads the ``ts3.0.grammar`` file that describes the TypeScript 3.0 grammar rules.
3. Hands both to ``GrammarParser`` and returns the resulting ``ASTNode`` tree.

The grammar file lives at ``code/grammars/typescript/ts3.0.grammar`` relative
to the repository root.

Locating the Grammar File
--------------------------

::

    parser.py → typescript_ts30_parser/ → src/ → typescript-ts3.0-parser/
    → python/ → packages/ → code/ → grammars/typescript/ts3.0.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from typescript_ts30_lexer import tokenize_ts30

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS30_GRAMMAR_PATH = GRAMMAR_DIR / "ts3.0.grammar"


def create_ts30_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for TypeScript 3.0 (2018).

    Args:
        source: The TypeScript 3.0 source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an ``ASTNode`` tree.

    Example::

        parser = create_ts30_parser('const x: unknown = 42;')
        ast = parser.parse()
    """
    tokens = tokenize_ts30(source)
    grammar = parse_parser_grammar(TS30_GRAMMAR_PATH.read_text(encoding="utf-8"))
    return GrammarParser(tokens, grammar)


def parse_ts30(source: str) -> ASTNode:
    """Parse TypeScript 3.0 source code and return an AST.

    Args:
        source: The TypeScript 3.0 source code to parse.

    Returns:
        An ``ASTNode`` representing the root of the parse tree (rule ``program``).

    Example::

        ast = parse_ts30('const x: unknown = 42;')
        print(ast.rule_name)  # "program"
    """
    parser = create_ts30_parser(source)
    return parser.parse()
