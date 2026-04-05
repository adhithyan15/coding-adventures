"""TypeScript 4.0 (2020) Parser — parses TypeScript 4.0 source code into ASTs.

Public API:

- ``create_ts40_parser(source)`` — creates a ``GrammarParser`` for TypeScript 4.0.
- ``parse_ts40(source)`` — parses source, returns an ``ASTNode`` tree.
"""

from __future__ import annotations

from typescript_ts40_parser.parser import create_ts40_parser, parse_ts40

__all__ = [
    "create_ts40_parser",
    "parse_ts40",
]
