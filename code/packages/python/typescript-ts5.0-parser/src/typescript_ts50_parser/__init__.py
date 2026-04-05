"""TypeScript 5.0 (2023) Parser — parses TypeScript 5.0 source code into ASTs.

Public API:

- ``create_ts50_parser(source)`` — creates a ``GrammarParser`` for TypeScript 5.0.
- ``parse_ts50(source)`` — parses source, returns an ``ASTNode`` tree.
"""

from __future__ import annotations

from typescript_ts50_parser.parser import create_ts50_parser, parse_ts50

__all__ = [
    "create_ts50_parser",
    "parse_ts50",
]
