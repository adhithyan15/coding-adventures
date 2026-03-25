"""SQL Parser — parses SQL text into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``sql.grammar`` file from the ``code/grammars/`` directory, tokenizes
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

Locating the Grammar File
--------------------------

The ``sql.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    parser.py
    └── sql_parser/       (parent)
        └── src/           (parent)
            └── sql-parser/ (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── sql.grammar

The module-level variable ``_sql_grammar_path`` can be overridden in tests
to point at an alternative path (or a non-existent path to exercise the
error branch). When it is the empty string ``""``, the auto-discovered path
is used.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from sql_lexer import tokenize_sql

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# We navigate from this file's location up to the repository root's
# grammars/ directory. The path is:
#   src/sql_parser/parser.py -> src/sql_parser -> src -> sql-parser
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
SQL_GRAMMAR_PATH = GRAMMAR_DIR / "sql.grammar"

# Module-level override for testing. When set to a non-empty string, that
# path is used instead of the auto-discovered SQL_GRAMMAR_PATH above.
# Tests can monkeypatch this to trigger error paths.
_sql_grammar_path: str = ""


def _resolve_grammar_path() -> Path:
    """Return the effective grammar file path.

    If ``_sql_grammar_path`` has been set to a non-empty string, use it.
    Otherwise, fall back to the auto-discovered ``SQL_GRAMMAR_PATH``.

    This indirection exists so tests can exercise the error path by pointing
    ``_sql_grammar_path`` at a non-existent file::

        import sql_parser.parser as p
        p._sql_grammar_path = "/no/such/file.grammar"
        # Now create_sql_parser() will raise FileNotFoundError
    """
    if _sql_grammar_path:
        return Path(_sql_grammar_path)
    return SQL_GRAMMAR_PATH


def create_sql_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for SQL text.

    This function:

    1. Tokenizes the source text using the SQL lexer.
    2. Reads and parses the ``sql.grammar`` file.
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
        FileNotFoundError: If the grammar file cannot be found.
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_sql_parser("SELECT id FROM users")
        ast = parser.parse("program")
    """
    grammar_path = _resolve_grammar_path()
    tokens = tokenize_sql(source)
    grammar = parse_parser_grammar(grammar_path.read_text())
    return GrammarParser(tokens, grammar)


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
        FileNotFoundError: If the grammar file cannot be found.
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
