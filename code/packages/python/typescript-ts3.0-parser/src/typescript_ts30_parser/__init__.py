"""TypeScript 3.0 (2018) Parser — parses TypeScript 3.0 source code into ASTs.

Public API:

- ``create_ts30_parser(source)`` — creates a ``GrammarParser`` for TypeScript 3.0.
- ``parse_ts30(source)`` — parses source, returns an ``ASTNode`` tree.
"""

from __future__ import annotations

from typescript_ts30_parser.parser import create_ts30_parser, parse_ts30

__all__ = [
    "create_ts30_parser",
    "parse_ts30",
]
