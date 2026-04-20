"""Oct Type Checker — type-checks Oct ASTs and annotates expression nodes.

This package is the third stage of the Oct compiler pipeline::

    Source text
        → Oct Lexer      (characters → tokens)
        → Oct Parser     (tokens → untyped ASTNode tree)
        → Type Checker   (untyped AST → typed AST)   ← this package
        → IR Compiler    (typed AST → IR)
        → …

Oct has exactly two value types:

- ``u8``   — unsigned 8-bit integer, range 0–255.
- ``bool`` — boolean (true/false).  Stored as u8 containing 0 or 1.

The checker annotates every expression node with a ``._oct_type`` attribute
so that the downstream IR compiler can read the type of any expression
without re-analysing the AST.

Typical usage::

    from oct_parser import parse_oct
    from oct_type_checker import check_oct

    ast = parse_oct("fn main() { let x: u8 = 42; }")
    result = check_oct(ast)
    if result.ok:
        print("OK — typed AST ready for IR compilation")
    else:
        for err in result.errors:
            print(f"{err.line}:{err.column}: {err.message}")
"""

from __future__ import annotations

from oct_type_checker.checker import OctTypeChecker, check_oct

__all__ = [
    "OctTypeChecker",
    "check_oct",
]
