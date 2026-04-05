"""TypeScript 2.0 (September 2016) Parser — parses TS 2.0 source into ASTs.

TypeScript 2.0 was released in September 2016. It upgraded the JavaScript
baseline to ECMAScript 2015 (ES6), adding ES2015 syntax, the ``never`` type,
non-nullable types, strict null checks, and mapped types.

Public API:

- ``create_ts20_parser(source)`` — creates a ``GrammarParser`` for TS 2.0.
- ``parse_ts20(source)`` — parses source, returns an ``ASTNode`` tree.
"""

from __future__ import annotations

from typescript_ts20_parser.parser import create_ts20_parser, parse_ts20

__all__ = [
    "create_ts20_parser",
    "parse_ts20",
]
