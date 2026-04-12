"""Dartmouth BASIC 1964 Parser — parses BASIC source into ASTs using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarParser``. It
tokenizes Dartmouth BASIC using the ``dartmouth_basic_lexer`` package, then
parses the token stream using the EBNF rules defined in ``dartmouth_basic.grammar``.

Dartmouth BASIC (1964) was created by John Kemeny and Thomas Kurtz at Dartmouth
College to make computing accessible to non-specialists. The original system ran
on a GE-225 mainframe connected to time-sharing teletypes. The language was
so influential that by 1980 virtually every microcomputer included a BASIC
interpreter — usually a descendant of the Microsoft BASIC written by Bill Gates
and Paul Allen for the Altair 8800 in 1975.

The grammar-driven parser approach means we describe the syntax once, in a plain
text grammar file, and the same parser engine handles every language that has a
grammar file. No BASIC-specific parsing logic lives in this module.

The result is a generic ``ASTNode`` tree — the same type used for JSON, Python,
JavaScript, and every other language in this codebase.

Usage::

    from dartmouth_basic_parser import parse_dartmouth_basic

    source = "10 LET X = 5\\n20 PRINT X\\n30 END\\n"
    ast = parse_dartmouth_basic(source)
    print(ast.rule_name)  # "program"

For lower-level access (e.g., to inspect grammar rules before parsing)::

    from dartmouth_basic_parser import create_dartmouth_basic_parser

    parser = create_dartmouth_basic_parser(source)
    ast = parser.parse()
"""

from dartmouth_basic_parser.parser import create_dartmouth_basic_parser, parse_dartmouth_basic

__all__ = [
    "create_dartmouth_basic_parser",
    "parse_dartmouth_basic",
]
