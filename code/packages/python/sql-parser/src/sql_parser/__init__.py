"""SQL Parser — parses SQL text into ASTs using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarParser``. It
tokenizes SQL using the ``sql-lexer`` package, then parses the token stream
using the EBNF rules defined in ``sql.grammar``.

The result is a generic ``ASTNode`` tree — the same type used for JSON, Python,
Ruby, JavaScript, and Starlark. This demonstrates the language-agnostic nature
of the grammar-driven approach.

Usage::

    from sql_parser import parse_sql

    ast = parse_sql("SELECT id, name FROM users WHERE age > 18")
    print(ast.rule_name)  # "program"
"""

from sql_parser.parser import create_sql_parser, parse_sql

__all__ = [
    "create_sql_parser",
    "parse_sql",
]
