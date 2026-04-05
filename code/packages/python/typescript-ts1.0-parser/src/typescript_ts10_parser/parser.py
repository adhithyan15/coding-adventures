"""TypeScript 1.0 (April 2014) Parser — parses TS 1.0 source into ASTs.

TypeScript 1.0 was the first public release of the TypeScript language,
announced at Microsoft's Build conference in April 2014. It added a static
type system to ECMAScript 5, introducing interfaces, classes, enums,
generics, namespaces, and ambient declarations.

Grammar Overview
-----------------

The TypeScript 1.0 grammar extends the ES5 grammar with:

- ``interface_declaration`` — ``interface Foo { x: string; }``
- ``type_alias_declaration`` — ``type Alias = string;``
- ``enum_declaration`` — ``enum Color { Red, Green, Blue }``
- ``namespace_declaration`` — ``namespace MyNS { }``
- ``ambient_declaration`` — ``declare var x: number;``
- ``ts_class_declaration`` — ``class Animal { name: string; }``
  (Note: uses ``ts_class_declaration`` in the TS grammar to avoid
  conflict with future ES2015 ``class_declaration``)
- ``type_annotation`` — the ``: type`` suffix on variables and parameters

Locating the Grammar File
--------------------------

::

    parser.py → typescript_ts10_parser/ → src/ → typescript-ts1.0-parser/
    → python/ → packages/ → code/ → grammars/typescript/ts1.0.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from typescript_ts10_lexer import tokenize_ts10

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS10_GRAMMAR_PATH = GRAMMAR_DIR / "ts1.0.grammar"


def create_ts10_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for TypeScript 1.0 (April 2014).

    TypeScript 1.0 extends ES5 with a static type system. The parser
    understands interfaces, classes, enums, namespaces, type annotations,
    generics, and ambient declarations.

    Args:
        source: The TypeScript 1.0 source code to parse.

    Returns:
        A ``GrammarParser`` instance configured with TS1.0 grammar rules.

    Example::

        parser = create_ts10_parser('interface Foo { x: string; }')
        ast = parser.parse()
    """
    tokens = tokenize_ts10(source)
    grammar = parse_parser_grammar(TS10_GRAMMAR_PATH.read_text(encoding="utf-8"))
    return GrammarParser(tokens, grammar)


def parse_ts10(source: str) -> ASTNode:
    """Parse TypeScript 1.0 source code and return an AST.

    Args:
        source: The TypeScript 1.0 source code to parse.

    Returns:
        An ``ASTNode`` representing the root ``program`` node of the AST.

    Example::

        ast = parse_ts10('var x: number = 1;')
        print(ast.rule_name)  # "program"
    """
    parser = create_ts10_parser(source)
    return parser.parse()
