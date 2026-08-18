"""SQL Parser — parses SQL text into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It uses
a pre-compiled ``PARSER_GRAMMAR`` (see ``sql_parser._grammar``), tokenizes
the input using the SQL lexer, and produces a generic ``ASTNode`` tree.

SQL (ANSI subset) is richer than JSON — it has dozens of statement forms,
nested expressions, and case-insensitive keywords. The grammar covers the
most common DML and DDL operations::

    program           = statement { ";" statement } [ ";" ] ;
    statement         = select_stmt | insert_stmt | update_stmt
                      | delete_stmt | create_table_stmt | drop_table_stmt ;
    select_stmt       = "SELECT" [ "DISTINCT" | "ALL" ] select_list
                        "FROM" table_ref { join_clause }
                        [ where_clause ] [ group_clause ] [ having_clause ]
                        [ order_clause ] [ limit_clause ] ;
    ...

Because the SQL lexer uses ``@case_insensitive true``, all keywords are
normalized to uppercase before the parser ever sees them. This means
``select``, ``SELECT``, and ``Select`` all produce the same token and will
parse identically.

The parser produces a tree of ``ASTNode`` objects where each node records
which grammar rule produced it and what children it matched. For example,
parsing ``SELECT 1 FROM t`` produces (simplified)::

    ASTNode(rule_name="program", children=[
        ASTNode(rule_name="statement", children=[
            ASTNode(rule_name="select_stmt", children=[
                Token(KEYWORD, 'SELECT'),
                ASTNode(rule_name="select_list", children=[...]),
                Token(KEYWORD, 'FROM'),
                ASTNode(rule_name="table_ref", children=[...]),
            ])
        ])
    ])

What This Module Provides
-------------------------

Two convenience functions:

- ``create_sql_parser(source)`` — tokenizes the source with ``sql_lexer``
  and creates a ``GrammarParser`` configured with the SQL grammar.
- ``parse_sql(source)`` — the all-in-one function. Pass in SQL text, get
  back an AST.

The SQL parser grammar is compiled into ``_grammar.py`` by the grammar-tools
compiler from ``code/grammars/sql/sql.grammar``. Importing the pre-built
``PARSER_GRAMMAR`` constant means no file I/O at startup and no runtime
grammar parsing overhead.

To regenerate after editing ``code/grammars/sql/sql.grammar``::

    grammar-tools compile-grammar code/grammars/sql/sql.grammar \\
        > code/packages/python/sql-parser/src/sql_parser/_grammar.py
"""

from __future__ import annotations

from lang_parser import ASTNode, GrammarParser
from sql_lexer import tokenize_sql

from sql_parser._grammar import PARSER_GRAMMAR


def create_sql_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for SQL text.

    This function:

    1. Tokenizes the source text using the SQL lexer.
    2. Uses the pre-compiled ``PARSER_GRAMMAR`` (from ``sql_parser._grammar``).
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    The SQL lexer normalizes all keywords to uppercase, so the grammar's
    quoted strings (``"SELECT"``, ``"FROM"``, etc.) will always match,
    regardless of how the user typed the keyword.

    Args:
        source: The SQL text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST rooted at the
        ``program`` rule (the first rule in the grammar).

    Raises:
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_sql_parser("SELECT id FROM users")
        ast = parser.parse("program")
    """
    tokens = tokenize_sql(source)
    return GrammarParser(tokens, PARSER_GRAMMAR)


def parse_sql(source: str) -> ASTNode:
    """Parse SQL text and return an AST.

    This is the main entry point for the SQL parser. Pass in a string of
    SQL text, and get back an ``ASTNode`` representing the complete parse
    tree.

    The root node always has ``rule_name="program"`` — the SQL grammar's
    start rule. A program is one or more semicolon-separated statements::

        program = statement { ";" statement } [ ";" ] ;

    Statement types recognized:

    - ``select_stmt`` — SELECT queries with optional WHERE, GROUP BY, HAVING,
      ORDER BY, LIMIT, OFFSET, and JOIN clauses.
    - ``insert_stmt`` — INSERT INTO ... VALUES (...).
    - ``update_stmt`` — UPDATE ... SET ... WHERE ...
    - ``delete_stmt`` — DELETE FROM ... WHERE ...
    - ``create_table_stmt`` — CREATE TABLE with column definitions and constraints.
    - ``drop_table_stmt`` — DROP TABLE with optional IF EXISTS.

    Because the SQL lexer is case-insensitive, ``select``, ``SELECT``, and
    ``Select`` all parse identically.

    Args:
        source: The SQL text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is ``"program"``.

    Raises:
        LexerError: If the source contains invalid characters.
        GrammarParseError: If the source has syntax errors according
            to the SQL grammar.

    Example::

        ast = parse_sql("SELECT id, name FROM users WHERE age > 18")
        # ASTNode(rule_name="program", children=[
        #     ASTNode(rule_name="statement", children=[
        #         ASTNode(rule_name="select_stmt", children=[...])
        #     ])
        # ])
    """
    parser = create_sql_parser(source)
    return parser.parse()
