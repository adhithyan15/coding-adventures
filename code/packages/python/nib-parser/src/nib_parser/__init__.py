"""Nib Parser — parses Nib source text into ASTs using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarParser``. It
tokenizes Nib source using the ``nib-lexer`` package, then parses the token
stream using the EBNF rules defined in ``nib.grammar``.

Nib (2024) is a safe, statically-typed toy language designed to compile to
Intel 4004 machine code. The name comes from "nibble" (4 bits), the native
word size of the Intel 4004 — the world's first commercial microprocessor
(1971). Nib brings static safety guarantees and structured control flow to
a CPU with 160 bytes of RAM and a 3-level hardware call stack.

The result of parsing is a generic ``ASTNode`` tree. The same parser engine
that handles ALGOL 60, Python, and JSON also handles Nib — only the grammar
file changes. This demonstrates the language-agnostic nature of the
grammar-driven approach.

Usage::

    from nib_parser import parse_nib

    ast = parse_nib("fn main() { let x: u4 = 5; }")
    print(ast.rule_name)  # "program"
"""

from __future__ import annotations

from nib_parser.parser import create_nib_parser, parse_nib

__all__ = [
    "create_nib_parser",
    "parse_nib",
]
