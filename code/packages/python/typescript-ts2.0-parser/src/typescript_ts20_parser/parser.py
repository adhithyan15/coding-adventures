"""TypeScript 2.0 (September 2016) Parser — parses TS 2.0 source into ASTs.

TypeScript 2.0 was released in September 2016. It upgraded the JavaScript
baseline from ECMAScript 5 to ECMAScript 2015 (ES6), adding many new grammar
rules for ES2015 constructs plus the TypeScript 2.0 type system additions.

What TS 2.0 Grammar Adds over TS 1.0
---------------------------------------

New ES2015 Grammar Rules (baseline upgrade):
- ``arrow_function`` — ``(x: string) => x.length``
- ``class_declaration`` — full ES2015 class syntax with extends/implements
- ``import_declaration`` — ``import { Foo } from "./foo"``
- ``export_declaration`` — ``export default class Foo {}``
- ``for_of_statement`` — ``for (const x of arr) {}``
- ``destructuring_assignment`` — ``const { x, y } = obj``
- ``template_literal`` — `` `Hello ${name}` ``
- ``generator_function`` — ``function* gen() { yield 1; }``

New TS 2.0 Type System Rules:
- ``never_type`` — ``never`` in type positions
- ``mapped_type`` — ``{ [K in keyof T]: T[K] }``
- ``conditional_type`` (added later, but grammar reserves it)
- ``import_type`` — ``import("./foo").Bar``

The ``never`` type represents the bottom of the type lattice — a value
of type ``never`` can never occur at runtime. It appears in:
1. Return types of functions that always throw or never return
2. Impossible branches in exhaustiveness checks
3. Bottom element of union types (``string | never`` simplifies to ``string``)

Locating the Grammar File
--------------------------

::

    parser.py → typescript_ts20_parser/ → src/ → typescript-ts2.0-parser/
    → python/ → packages/ → code/ → grammars/typescript/ts2.0.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from typescript_ts20_lexer import tokenize_ts20

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS20_GRAMMAR_PATH = GRAMMAR_DIR / "ts2.0.grammar"


def create_ts20_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for TypeScript 2.0 (September 2016).

    TypeScript 2.0 extends TS 1.0 with ES2015 syntax and new type system
    features including the ``never`` type, non-nullable types, and mapped
    types.

    Args:
        source: The TypeScript 2.0 source code to parse.

    Returns:
        A ``GrammarParser`` instance configured with TS2.0 grammar rules.

    Example::

        parser = create_ts20_parser('const x: string | never = "hello";')
        ast = parser.parse()
    """
    tokens = tokenize_ts20(source)
    grammar = parse_parser_grammar(TS20_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_ts20(source: str) -> ASTNode:
    """Parse TypeScript 2.0 source code and return an AST.

    Args:
        source: The TypeScript 2.0 source code to parse.

    Returns:
        An ``ASTNode`` representing the root ``program`` node of the AST.

    Example::

        ast = parse_ts20('const x: never = undefined as never;')
        print(ast.rule_name)  # "program"
    """
    parser = create_ts20_parser(source)
    return parser.parse()
