"""ECMAScript 1 (1997) Parser — parses ES1 JavaScript into ASTs.

Public API:

- ``create_es1_parser(source)`` — creates a ``GrammarParser`` for ES1.
- ``parse_es1(source)`` — parses source, returns an ``ASTNode`` tree.
"""

from __future__ import annotations

from ecmascript_es1_parser.parser import create_es1_parser, parse_es1

__all__ = [
    "create_es1_parser",
    "parse_es1",
]
