"""ECMAScript 5 (2009) Parser — parses ES5 JavaScript into ASTs.

Public API:

- ``create_es5_parser(source)`` — creates a ``GrammarParser`` for ES5.
- ``parse_es5(source)`` — parses source, returns an ``ASTNode`` tree.
"""

from __future__ import annotations

from ecmascript_es5_parser.parser import create_es5_parser, parse_es5

__all__ = [
    "create_es5_parser",
    "parse_es5",
]
