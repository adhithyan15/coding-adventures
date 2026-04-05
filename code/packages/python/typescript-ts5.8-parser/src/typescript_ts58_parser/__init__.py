"""TypeScript 5.8 (2025) Parser — parses TypeScript 5.8 source code into ASTs.

Public API:

- ``create_ts58_parser(source)`` — creates a ``GrammarParser`` for TypeScript 5.8.
- ``parse_ts58(source)`` — parses source, returns an ``ASTNode`` tree.
"""

from __future__ import annotations

from typescript_ts58_parser.parser import create_ts58_parser, parse_ts58

__all__ = [
    "create_ts58_parser",
    "parse_ts58",
]
