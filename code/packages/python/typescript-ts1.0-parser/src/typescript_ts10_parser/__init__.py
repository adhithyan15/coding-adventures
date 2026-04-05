"""TypeScript 1.0 (April 2014) Parser — parses TS 1.0 source into ASTs.

TypeScript 1.0 was the first public release of TypeScript, announced at
Microsoft's Build conference in April 2014. It extended ECMAScript 5 with
a static type system including interfaces, classes, enums, generics, and
namespaces.

Public API:

- ``create_ts10_parser(source)`` — creates a ``GrammarParser`` for TS 1.0.
- ``parse_ts10(source)`` — parses source, returns an ``ASTNode`` tree.
"""

from __future__ import annotations

from typescript_ts10_parser.parser import create_ts10_parser, parse_ts10

__all__ = [
    "create_ts10_parser",
    "parse_ts10",
]
