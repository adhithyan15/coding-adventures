"""SQL Lexer — tokenizes SQL text using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``sql.tokens`` file from the ``code/grammars/`` directory and creates a
lexer configured for ANSI SQL tokenization.

SQL is a case-insensitive query language. Unlike JSON, SQL has:

- **Keywords.** SQL has dozens of reserved words: ``SELECT``, ``FROM``,
  ``WHERE``, ``JOIN``, ``NULL``, ``TRUE``, ``FALSE``, and more. These are
  reclassified from NAME tokens to KEYWORD tokens by the lexer.
- **Case insensitivity.** The grammar specifies ``# @case_insensitive true``,
  which means keyword values are normalized to uppercase. ``select``,
  ``SELECT``, and ``Select`` all produce ``KEYWORD("SELECT")``.
- **Multiple operator spellings.** Both ``!=`` and ``<>`` produce
  ``NOT_EQUALS`` tokens (``NEQ_ANSI`` is aliased). Compound operators
  ``<=``, ``>=`` are matched before ``<``, ``>`` to ensure longest-match.
- **String literals.** SQL uses *single* quotes for string literals::

      'hello world'   →   STRING("hello world")   (quotes stripped)

- **Quoted identifiers.** Backtick identifiers allow spaces and reserved
  words as column/table names. They alias to NAME, but the backticks are
  preserved in the value (the lexer only strips quotes for STRING patterns)::

      `my table`   →   NAME("`my table`")

- **Comments.** SQL supports two comment styles, both silently skipped:
    - ``-- line comment`` (from ``--`` to end of line)
    - ``/* block comment */`` (spanning any number of lines)

Truth Table — Case Normalization
---------------------------------

+----------------+---------------+---------------------+
| Input text     | Token type    | Token value         |
+================+===============+=====================+
| ``select``     | KEYWORD       | ``SELECT``          |
| ``SELECT``     | KEYWORD       | ``SELECT``          |
| ``Select``     | KEYWORD       | ``SELECT``          |
| ``from``       | KEYWORD       | ``FROM``            |
| ``users``      | NAME          | ``users``           |
| ``42``         | NUMBER        | ``42``              |
| ``3.14``       | NUMBER        | ``3.14``            |
| ``'hello'``    | STRING        | ``hello``           |
| `` `col` ``    | NAME          | `` `col` ``         |
| ``<=``         | LESS_EQUALS   | ``<=``              |
| ``<>``         | NOT_EQUALS    | ``<>``              |
| ``!=``         | NOT_EQUALS    | ``!=``              |
+----------------+---------------+---------------------+

What This Module Provides
-------------------------

Two convenience functions:

- ``create_sql_lexer(source)`` — creates a ``GrammarLexer`` configured for
  SQL. Use this when you want to control the tokenization process yourself.
- ``tokenize_sql(source)`` — the all-in-one function. Pass in SQL text,
  get back a list of tokens. This is the function most callers want.

Locating the Grammar File
--------------------------

The ``sql.tokens`` file lives in the ``code/grammars/`` directory at the
root of the coding-adventures repository. We locate it relative to this
module's file path using ``pathlib.Path``::

    tokenizer.py
    └── sql_lexer/        (parent)
        └── src/           (parent)
            └── sql-lexer/  (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── sql.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# We navigate from this file's location up to the repository root's
# grammars/ directory. The path is:
#   src/sql_lexer/tokenizer.py -> src/sql_lexer -> src -> sql-lexer
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
SQL_TOKENS_PATH = GRAMMAR_DIR / "sql.tokens"


def create_sql_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for SQL text.

    This function reads the ``sql.tokens`` file, parses it into a
    ``TokenGrammar``, and creates a ``GrammarLexer`` ready to tokenize
    the given source text.

    The grammar's ``# @case_insensitive true`` directive causes the lexer
    to normalize all keyword values to uppercase. Identifiers and other
    non-keyword tokens preserve their original casing.

    Args:
        source: The SQL text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with SQL token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Raises:
        FileNotFoundError: If the ``sql.tokens`` file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

    Example::

        lexer = create_sql_lexer("SELECT id FROM users")
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(SQL_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_sql(source: str) -> list[Token]:
    """Tokenize SQL text and return a list of tokens.

    This is the main entry point for the SQL lexer. Pass in a string of
    SQL text, and get back a flat list of ``Token`` objects. The list
    always ends with an ``EOF`` token.

    Keywords are normalized to uppercase regardless of their original casing
    in the source text. This implements the ANSI SQL standard rule that
    keywords are case-insensitive.

    The token types you will see include:

    - **KEYWORD** — a SQL reserved word (``SELECT``, ``FROM``, ``WHERE``,
      ``NULL``, ``TRUE``, ``FALSE``, etc.). Value is always uppercase.
    - **NAME** — an identifier (table name, column name, etc.)
    - **NUMBER** — an integer or decimal number literal
    - **STRING** — a single-quoted string literal (quotes stripped)
    - **EQUALS** / **NOT_EQUALS** / **LESS_THAN** / **GREATER_THAN** /
      **LESS_EQUALS** / **GREATER_EQUALS** — comparison operators
    - **PLUS** / **MINUS** / **STAR** / **SLASH** / **PERCENT** —
      arithmetic operators
    - **LPAREN** / **RPAREN** — parentheses
    - **COMMA** / **SEMICOLON** / **DOT** — punctuation
    - **EOF** — end of input

    Args:
        source: The SQL text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Raises:
        FileNotFoundError: If the ``sql.tokens`` file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in the SQL grammar.

    Example::

        tokens = tokenize_sql("SELECT id FROM users WHERE age >= 18")
        # [Token(KEYWORD, 'SELECT'), Token(NAME, 'id'), Token(KEYWORD, 'FROM'),
        #  Token(NAME, 'users'), Token(KEYWORD, 'WHERE'), Token(NAME, 'age'),
        #  Token(GREATER_EQUALS, '>='), Token(NUMBER, '18'), Token(EOF, '')]
    """
    lexer = create_sql_lexer(source)
    return lexer.tokenize()
