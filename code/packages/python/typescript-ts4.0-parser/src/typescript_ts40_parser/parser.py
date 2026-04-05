r"""TypeScript 4.0 (2020) Parser — parses TypeScript 4.0 source code into ASTs.

TypeScript 4.0 introduced variadic tuple types, labeled tuple elements,
template literal types, and short-circuit assignment operators on an ES2020
baseline. This parser produces ``ASTNode`` trees from TypeScript 4.0 source.

Architecture
------------

This module is a thin wrapper around the generic ``GrammarParser``. It:

1. Tokenizes the source using ``tokenize_ts40`` from the sibling lexer package.
2. Loads the ``ts4.0.grammar`` file that describes the TypeScript 4.0 grammar rules.
3. Hands both to ``GrammarParser`` and returns the resulting ``ASTNode`` tree.

The grammar file lives at ``code/grammars/typescript/ts4.0.grammar`` relative
to the repository root.

TypeScript 4.0 Grammar Highlights
-----------------------------------

Variadic tuple types allow generic type-level array concatenation::

    type Concat<T extends unknown[], U extends unknown[]> = [...T, ...U];

Labeled tuple elements give tuple positions documentation names::

    type HttpRequest = [method: string, url: string, body?: unknown];

Template literal types build string types at compile time::

    type EventName<T extends string> = \`on${Capitalize<T>}\`;

Short-circuit assignment operators ``&&=``, ``||=``, ``??=`` assign only
when the left side meets a condition (truthy, falsy, or nullish respectively).

Locating the Grammar File
--------------------------

::

    parser.py → typescript_ts40_parser/ → src/ → typescript-ts4.0-parser/
    → python/ → packages/ → code/ → grammars/typescript/ts4.0.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from typescript_ts40_lexer import tokenize_ts40

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS40_GRAMMAR_PATH = GRAMMAR_DIR / "ts4.0.grammar"


def create_ts40_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for TypeScript 4.0 (2020).

    Args:
        source: The TypeScript 4.0 source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an ``ASTNode`` tree.

    Example::

        parser = create_ts40_parser('type Pair = [first: string, second: number];')
        ast = parser.parse()
    """
    tokens = tokenize_ts40(source)
    grammar = parse_parser_grammar(TS40_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_ts40(source: str) -> ASTNode:
    """Parse TypeScript 4.0 source code and return an AST.

    Args:
        source: The TypeScript 4.0 source code to parse.

    Returns:
        An ``ASTNode`` representing the root of the parse tree (rule ``program``).

    Example::

        ast = parse_ts40('type Pair = [first: string, second: number];')
        print(ast.rule_name)  # "program"
    """
    parser = create_ts40_parser(source)
    return parser.parse()
