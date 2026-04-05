"""ECMAScript 3 (1999) Parser — parses ES3 JavaScript into ASTs.

Public API:

- ``create_es3_parser(source)`` — creates a ``GrammarParser`` for ES3.
- ``parse_es3(source)`` — parses source, returns an ``ASTNode`` tree.
"""

from __future__ import annotations

from ecmascript_es3_parser.parser import create_es3_parser, parse_es3

__all__ = [
    "create_es3_parser",
    "parse_es3",
]
