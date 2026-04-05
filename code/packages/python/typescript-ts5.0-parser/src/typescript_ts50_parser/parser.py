"""TypeScript 5.0 (2023) Parser — parses TypeScript 5.0 into ASTs.

TypeScript 5.0 adds standard TC39 decorators, ``const`` type parameters,
the ``accessor`` keyword for auto-accessor class members, and the ``satisfies``
operator. It targets ES2022 as its baseline (class fields, private members,
static initialization blocks).

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``ts5.0.grammar`` file, tokenizes the source with the TS 5.0 lexer, and
produces an ``ASTNode`` tree.

Grammar Highlights
-------------------

The TS 5.0 grammar covers the full TypeScript type system on top of ES2022:

- ``interface_declaration`` — TypeScript interface definitions
- ``type_alias_declaration`` — ``type Alias = ...`` declarations
- ``enum_declaration`` — TypeScript enums (regular and ``const`` enums)
- ``ts_class_declaration`` — classes with optional decorators and type params
- ``type_parameters`` — generic parameter lists ``<T, U extends V>``
- ``using_declaration`` / ``await_using_declaration`` — TS 5.2 resource management
- All ES2022 statements, expressions, and destructuring patterns
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from typescript_ts50_lexer import tokenize_ts50

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS50_GRAMMAR_PATH = GRAMMAR_DIR / "ts5.0.grammar"


def create_ts50_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for TypeScript 5.0 (2023).

    Args:
        source: The TypeScript 5.0 source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.

    Example::

        parser = create_ts50_parser('interface Foo { x: number; }')
        ast = parser.parse()
    """
    tokens = tokenize_ts50(source)
    grammar = parse_parser_grammar(TS50_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_ts50(source: str) -> ASTNode:
    """Parse TypeScript 5.0 source code and return an AST.

    Args:
        source: The TypeScript 5.0 source code to parse.

    Returns:
        An ``ASTNode`` rooted at ``program``.

    Example::

        ast = parse_ts50('const x: number = 1;')
        assert ast.rule_name == "program"
    """
    parser = create_ts50_parser(source)
    return parser.parse()
