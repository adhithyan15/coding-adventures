"""Oct Parser — parses Oct source text into ASTs using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarParser``. It
tokenizes Oct source using the ``oct-lexer`` package, then parses the token
stream using the EBNF rules defined in ``oct.grammar``, producing a generic
``ASTNode`` tree.

Oct (2026) is a safe, statically-typed toy language designed to compile to
Intel 8008 machine code. The name comes from *octet* — the networking term for
exactly 8 bits, the native word of the Intel 8008 ALU (1972). Oct brings
static safety guarantees and structured control flow to a CPU with a 16 KB
address space and a 7-level push-down call stack.

The same parser engine that handles Nib, ALGOL 60, Python, and JSON also
handles Oct — only the grammar file changes. This demonstrates the language-
agnostic nature of the grammar-driven approach.

Usage::

    from oct_parser import parse_oct

    ast = parse_oct("fn main() { let x: u8 = 0xFF; }")
    print(ast.rule_name)  # "program"
"""

from __future__ import annotations

from oct_parser.parser import create_oct_parser, parse_oct

__all__ = [
    "create_oct_parser",
    "parse_oct",
]
