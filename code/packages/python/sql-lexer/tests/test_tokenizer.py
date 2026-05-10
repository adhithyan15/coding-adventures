"""Tests for the SQL lexer thin wrapper.

These tests verify that the grammar-driven lexer, configured with
``sql.tokens``, correctly tokenizes ANSI SQL text.

The tests mirror the Go sql-lexer test suite (``sql_lexer_test.go``) to
ensure both implementations produce identical results.

Key behaviours under test:

- **Case-insensitive keywords**: ``select``, ``SELECT``, and ``Select`` all
  produce ``KEYWORD("SELECT")``. This is required by the ANSI SQL standard.
- **Operators**: both ``!=`` and ``<>`` produce ``NOT_EQUALS``; compound
  operators ``<=``, ``>=`` are matched as single tokens (longest-match rule).
- **Strings**: single-quoted ``'hello'`` → ``STRING("hello")`` (quotes stripped).
- **Quoted identifiers**: backtick `` `col` `` → ``NAME("`col`")`` (backticks kept).
- **Comments**: both ``--`` line comments and ``/* */`` block comments are silently
  skipped and produce no tokens.
"""

from __future__ import annotations

import pytest
from lexer import GrammarLexer, Token

from sql_lexer import create_sql_lexer, tokenize_sql

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def non_eof(source: str) -> list[Token]:
    """Tokenize and return all tokens except EOF."""
    tokens = tokenize_sql(source)
    return [
        t for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


def token_type_names(source: str) -> list[str]:
    """Tokenize and return just the type names (excluding EOF)."""
    return [
        t.type if isinstance(t.type, str) else t.type.name
        for t in non_eof(source)
    ]


def token_values(source: str) -> list[str]:
    """Tokenize and return just the values (excluding EOF)."""
    return [t.value for t in non_eof(source)]


def type_name(tok: Token) -> str:
    """Return the type name string for a token."""
    return tok.type if isinstance(tok.type, str) else tok.type.name


# ---------------------------------------------------------------------------
# TestFactory — create_sql_lexer factory function
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_sql_lexer factory function."""

    def test_returns_grammar_lexer(self) -> None:
        """create_sql_lexer should return a GrammarLexer instance."""
        lexer = create_sql_lexer("SELECT 1")
        assert isinstance(lexer, GrammarLexer)

    def test_factory_is_not_none(self) -> None:
        """create_sql_lexer should return a non-None lexer."""
        lexer = create_sql_lexer("SELECT 1")
        assert lexer is not None

    def test_factory_produces_tokens(self) -> None:
        """The factory-created lexer should produce valid tokens."""
        lexer = create_sql_lexer("SELECT 1")
        tokens = lexer.tokenize()
        # At minimum: KEYWORD(SELECT), NUMBER(1), EOF
        assert len(tokens) >= 2
        last = tokens[-1]
        assert (last.type if isinstance(last.type, str) else last.type.name) == "EOF"


# ---------------------------------------------------------------------------
# TestBasicSelect — SELECT id FROM users
# ---------------------------------------------------------------------------


class TestBasicSelect:
    """Verify basic SELECT statement tokenization.

    Input: SELECT id FROM users
    Expected: KEYWORD("SELECT"), NAME("id"), KEYWORD("FROM"), NAME("users"), EOF

    This is the foundation test — if keywords and names are correctly
    distinguished, all further SQL statement tests can build on it.
    """

    def test_select_token_types(self) -> None:
        """SELECT id FROM users produces keyword-name-keyword-name."""
        types = token_type_names("SELECT id FROM users")
        assert types == ["KEYWORD", "NAME", "KEYWORD", "NAME"]

    def test_select_token_values(self) -> None:
        """Keywords come out uppercase; names keep their original case."""
        tokens = non_eof("SELECT id FROM users")
        assert tokens[0].value == "SELECT"
        assert tokens[1].value == "id"
        assert tokens[2].value == "FROM"
        assert tokens[3].value == "users"

    def test_select_keyword_type(self) -> None:
        """SELECT token has KEYWORD type."""
        tokens = non_eof("SELECT id FROM users")
        assert type_name(tokens[0]) == "KEYWORD"

    def test_name_type(self) -> None:
        """Identifiers have NAME type."""
        tokens = non_eof("SELECT id FROM users")
        assert type_name(tokens[1]) == "NAME"
        assert type_name(tokens[3]) == "NAME"


# ---------------------------------------------------------------------------
# TestCaseInsensitiveKeywords
# ---------------------------------------------------------------------------


class TestCaseInsensitiveKeywords:
    """SQL keywords are case-insensitive per the ANSI standard.

    The grammar's ``# @case_insensitive true`` directive normalizes all
    keyword values to uppercase. The token type is always KEYWORD.

    Truth table:
    +------------+---------------+---------------+
    | Input      | Type          | Value         |
    +============+===============+===============+
    | select     | KEYWORD       | SELECT        |
    | SELECT     | KEYWORD       | SELECT        |
    | Select     | KEYWORD       | SELECT        |
    | from       | KEYWORD       | FROM          |
    | WHERE      | KEYWORD       | WHERE         |
    | Insert     | KEYWORD       | INSERT        |
    +------------+---------------+---------------+
    """

    @pytest.mark.parametrize("input_kw,expected_val", [
        ("select", "SELECT"),
        ("SELECT", "SELECT"),
        ("Select", "SELECT"),
        ("from", "FROM"),
        ("WHERE", "WHERE"),
        ("Insert", "INSERT"),
    ])
    def test_keyword_normalized_to_uppercase(
        self, input_kw: str, expected_val: str
    ) -> None:
        """Every keyword spelling produces uppercase KEYWORD token."""
        tokens = non_eof(input_kw)
        assert len(tokens) == 1
        assert type_name(tokens[0]) == "KEYWORD"
        assert tokens[0].value == expected_val


# ---------------------------------------------------------------------------
# TestNumberTokens
# ---------------------------------------------------------------------------


class TestNumberTokens:
    """Verify integer and decimal number literal tokenization.

    SQL uses numbers in WHERE clauses, LIMIT/OFFSET clauses, and
    arithmetic expressions.
    """

    @pytest.mark.parametrize("source", ["42", "3.14", "0", "100"])
    def test_number_type(self, source: str) -> None:
        """Each numeric literal tokenizes as a single NUMBER token."""
        tokens = non_eof(source)
        assert len(tokens) == 1
        assert type_name(tokens[0]) == "NUMBER"

    @pytest.mark.parametrize("source", ["42", "3.14", "0", "100"])
    def test_number_value_preserved(self, source: str) -> None:
        """NUMBER token value matches the source text exactly."""
        tokens = non_eof(source)
        assert tokens[0].value == source

    def test_integer(self) -> None:
        """Simple integer: 42 → NUMBER("42")."""
        tokens = non_eof("42")
        assert type_name(tokens[0]) == "NUMBER"
        assert tokens[0].value == "42"

    def test_decimal(self) -> None:
        """Decimal number: 3.14 → NUMBER("3.14")."""
        tokens = non_eof("3.14")
        assert type_name(tokens[0]) == "NUMBER"
        assert tokens[0].value == "3.14"

    def test_zero(self) -> None:
        """Zero: 0 → NUMBER("0")."""
        tokens = non_eof("0")
        assert tokens[0].value == "0"


# ---------------------------------------------------------------------------
# TestStringTokens
# ---------------------------------------------------------------------------


class TestStringTokens:
    """Verify single-quoted string literal tokenization.

    SQL uses single quotes for string literals. The grammar aliases
    STRING_SQ → STRING, so the token type is STRING. The surrounding
    single quotes are stripped from the token value.
    """

    def test_simple_string(self) -> None:
        """'hello world' → STRING("hello world") (quotes stripped)."""
        tokens = non_eof("'hello world'")
        assert len(tokens) == 1
        assert type_name(tokens[0]) == "STRING"
        assert tokens[0].value == "hello world"

    def test_string_quotes_stripped(self) -> None:
        """The surrounding single quotes must not appear in the token value."""
        tokens = non_eof("'Ada'")
        assert tokens[0].value == "Ada"

    def test_empty_string(self) -> None:
        """An empty string: '' → STRING("")."""
        tokens = non_eof("''")
        assert len(tokens) == 1
        assert type_name(tokens[0]) == "STRING"
        assert tokens[0].value == ""

    def test_string_with_spaces(self) -> None:
        """A string containing spaces tokenizes as one STRING token."""
        tokens = non_eof("'hello world'")
        assert len(tokens) == 1


# ---------------------------------------------------------------------------
# TestOperators
# ---------------------------------------------------------------------------


class TestOperators:
    """Verify SQL comparison and arithmetic operator tokenization.

    Important: compound operators ``<=``, ``>=`` must be matched as single
    tokens (longest-match rule), not as two separate tokens. Both ``!=`` and
    ``<>`` must produce NOT_EQUALS (NEQ_ANSI is aliased to NOT_EQUALS).

    Token type names are checked via ``token.type_name`` — the exact string
    stored in the grammar (e.g., "GREATER_EQUALS", not an enum value).
    """

    @pytest.mark.parametrize("source,expected_type", [
        ("=", "EQUALS"),
        ("!=", "NOT_EQUALS"),
        ("<>", "NOT_EQUALS"),
        ("<", "LESS_THAN"),
        (">", "GREATER_THAN"),
        ("<=", "LESS_EQUALS"),
        (">=", "GREATER_EQUALS"),
        ("+", "PLUS"),
        ("-", "MINUS"),
        ("*", "STAR"),
        ("/", "SLASH"),
        ("%", "PERCENT"),
    ])
    def test_operator_type_name(self, source: str, expected_type: str) -> None:
        """Each operator input produces exactly one token with the right type name."""
        tokens = non_eof(source)
        assert len(tokens) == 1, f"Expected 1 token for {source!r}, got {len(tokens)}"
        assert type_name(tokens[0]) == expected_type, (
            f"For {source!r}: expected {expected_type!r}, got {type_name(tokens[0])!r}"
        )

    def test_less_equals_is_single_token(self) -> None:
        """<= must be a single LESS_EQUALS token, not two separate tokens."""
        tokens = non_eof("<=")
        assert len(tokens) == 1
        assert type_name(tokens[0]) == "LESS_EQUALS"

    def test_greater_equals_is_single_token(self) -> None:
        """>= must be a single GREATER_EQUALS token, not two separate tokens."""
        tokens = non_eof(">=")
        assert len(tokens) == 1
        assert type_name(tokens[0]) == "GREATER_EQUALS"

    def test_neq_ansi_aliases_to_not_equals(self) -> None:
        """<> (ANSI not-equals) must produce NOT_EQUALS type, same as !=."""
        tokens_ansi = non_eof("<>")
        tokens_alt = non_eof("!=")
        assert type_name(tokens_ansi[0]) == "NOT_EQUALS"
        assert type_name(tokens_alt[0]) == "NOT_EQUALS"


# ---------------------------------------------------------------------------
# TestPunctuation
# ---------------------------------------------------------------------------


class TestPunctuation:
    """Verify punctuation token recognition.

    SQL uses punctuation for argument lists (COMMA), statement terminators
    (SEMICOLON), schema-qualified names (DOT), and subexpressions (LPAREN,
    RPAREN).
    """

    @pytest.mark.parametrize("source,expected_type", [
        ("(", "LPAREN"),
        (")", "RPAREN"),
        (",", "COMMA"),
        (";", "SEMICOLON"),
        (".", "DOT"),
    ])
    def test_punctuation_type_name(self, source: str, expected_type: str) -> None:
        """Each punctuation character produces one token with the right type name."""
        tokens = non_eof(source)
        assert len(tokens) == 1
        assert type_name(tokens[0]) == expected_type


# ---------------------------------------------------------------------------
# TestLineComments
# ---------------------------------------------------------------------------


class TestLineComments:
    """Verify that -- line comments are silently skipped.

    Line comments run from ``--`` to the end of the line. They must not
    appear in the token stream at all.
    """

    def test_line_comment_removed(self) -> None:
        """Comment text must not appear in the token stream."""
        source = "SELECT id -- pick the id column\nFROM users"
        tokens = non_eof(source)
        # Expected: KEYWORD(SELECT), NAME(id), KEYWORD(FROM), NAME(users)
        assert len(tokens) == 4, f"Expected 4 tokens (comment skipped), got {len(tokens)}: {tokens}"
        assert tokens[0].value == "SELECT"
        assert tokens[1].value == "id"
        assert tokens[2].value == "FROM"
        assert tokens[3].value == "users"

    def test_line_comment_at_end(self) -> None:
        """A trailing -- comment produces no extra tokens."""
        source = "SELECT 1 -- trailing comment"
        tokens = non_eof(source)
        assert len(tokens) == 2  # KEYWORD(SELECT), NUMBER(1)
        assert tokens[0].value == "SELECT"
        assert tokens[1].value == "1"


# ---------------------------------------------------------------------------
# TestBlockComments
# ---------------------------------------------------------------------------


class TestBlockComments:
    """Verify that /* block comments */ are silently skipped.

    Block comments can span multiple lines. They must not appear in the
    token stream.
    """

    def test_block_comment_removed(self) -> None:
        """Block comment text must not appear in the token stream."""
        source = "SELECT /* all columns */ * FROM t"
        tokens = non_eof(source)
        # Expected: KEYWORD(SELECT), STAR(*), KEYWORD(FROM), NAME(t)
        assert len(tokens) == 4, f"Expected 4 tokens (block comment skipped), got {len(tokens)}: {tokens}"
        assert tokens[0].value == "SELECT"
        assert type_name(tokens[1]) == "STAR"
        assert tokens[2].value == "FROM"

    def test_multiline_block_comment(self) -> None:
        """A block comment spanning multiple lines is skipped entirely."""
        source = "SELECT\n/* this is\na multiline comment\n*/\n1"
        tokens = non_eof(source)
        assert len(tokens) == 2  # KEYWORD(SELECT), NUMBER(1)


# ---------------------------------------------------------------------------
# TestWhereClause
# ---------------------------------------------------------------------------


class TestWhereClause:
    """Verify tokenization of a WHERE clause with a comparison expression.

    This test exercises identifier, keyword, operator, and number tokenization
    together, mirroring TestTokenizeSQLWhereClause in the Go test suite.
    """

    def test_where_clause_types(self) -> None:
        """WHERE age >= 18 produces keyword-name-operator-number."""
        tokens = non_eof("WHERE age >= 18")
        assert len(tokens) == 4
        assert type_name(tokens[0]) == "KEYWORD"
        assert tokens[0].value == "WHERE"
        assert type_name(tokens[1]) == "NAME"
        assert tokens[1].value == "age"
        assert type_name(tokens[2]) == "GREATER_EQUALS"
        assert type_name(tokens[3]) == "NUMBER"
        assert tokens[3].value == "18"


# ---------------------------------------------------------------------------
# TestQualifiedNames
# ---------------------------------------------------------------------------


class TestQualifiedNames:
    """Verify schema-qualified name tokenization.

    ``schema.orders`` must produce three tokens: NAME DOT NAME.
    The dot is a separate PUNCTUATION/DOT token; the parser combines them
    into qualified references.

    Note: We use ``schema.orders`` rather than ``schema.table`` because
    ``table`` is a SQL keyword; it would produce NAME DOT KEYWORD.
    """

    def test_qualified_name_is_three_tokens(self) -> None:
        """schema.orders → NAME("schema") DOT NAME("orders")."""
        tokens = non_eof("schema.orders")
        assert len(tokens) == 3, f"Expected 3 tokens, got {len(tokens)}: {tokens}"
        assert type_name(tokens[0]) == "NAME"
        assert tokens[0].value == "schema"
        assert type_name(tokens[1]) == "DOT"
        assert type_name(tokens[2]) == "NAME"
        assert tokens[2].value == "orders"


# ---------------------------------------------------------------------------
# TestQuotedIdentifiers
# ---------------------------------------------------------------------------


class TestQuotedIdentifiers:
    """Verify that backtick-quoted identifiers alias to NAME.

    QUOTED_ID = /`[^`]+`/ -> NAME
    The backtick quotes are preserved in the value because the lexer
    only strips quotes for patterns whose name or alias contains "STRING".
    """

    def test_quoted_identifier_type_is_name(self) -> None:
        """Backtick identifier aliases to NAME token type."""
        tokens = non_eof("`my table`")
        assert len(tokens) == 1
        assert type_name(tokens[0]) == "NAME"

    def test_quoted_identifier_value_includes_backticks(self) -> None:
        """Backtick quotes are preserved in the token value."""
        tokens = non_eof("`my table`")
        assert tokens[0].value == "`my table`"

    def test_quoted_identifier_allows_spaces(self) -> None:
        """A backtick identifier with spaces tokenizes as one NAME token."""
        tokens = non_eof("`column with spaces`")
        assert len(tokens) == 1
        assert type_name(tokens[0]) == "NAME"


# ---------------------------------------------------------------------------
# TestFullSelectStatement
# ---------------------------------------------------------------------------


class TestFullSelectStatement:
    """Integration test: a complete SELECT statement with multiple clauses.

    This mirrors TestTokenizeSQLFullSelectStatement in the Go test suite.
    It verifies that a realistic SQL query tokenizes correctly end-to-end.
    """

    def test_full_select_first_token(self) -> None:
        """First token of a SELECT statement must be KEYWORD('SELECT')."""
        source = "SELECT id, name FROM users WHERE active = TRUE ORDER BY name ASC LIMIT 10"
        tokens = non_eof(source)
        assert len(tokens) > 0
        assert tokens[0].value == "SELECT"
        assert type_name(tokens[0]) == "KEYWORD"

    def test_full_select_all_keywords_uppercase(self) -> None:
        """All keywords in the query must be uppercase regardless of input casing."""
        source = "SELECT id, name FROM users WHERE active = TRUE ORDER BY name ASC LIMIT 10"
        tokens = non_eof(source)
        keywords = [t.value for t in tokens if type_name(t) == "KEYWORD"]
        expected = ["SELECT", "FROM", "WHERE", "TRUE", "ORDER", "BY", "ASC", "LIMIT"]
        assert keywords == expected

    def test_full_select_token_count(self) -> None:
        """A simple SELECT returns a reasonable number of tokens."""
        source = "SELECT id FROM users"
        tokens = non_eof(source)
        assert len(tokens) == 4


# ---------------------------------------------------------------------------
# TestNullTrueFalse
# ---------------------------------------------------------------------------


class TestNullTrueFalse:
    """Verify that NULL, TRUE, and FALSE are KEYWORD tokens.

    These look like identifiers syntactically but are reserved SQL keywords.
    They are case-insensitive: null, NULL, Null all produce KEYWORD("NULL").
    """

    @pytest.mark.parametrize("source", ["NULL", "null", "TRUE", "true", "FALSE", "false"])
    def test_is_keyword(self, source: str) -> None:
        """NULL/TRUE/FALSE in any case must be KEYWORD tokens."""
        tokens = non_eof(source)
        assert len(tokens) == 1
        assert type_name(tokens[0]) == "KEYWORD"

    def test_null_uppercase(self) -> None:
        """null normalizes to KEYWORD('NULL')."""
        tokens = non_eof("null")
        assert tokens[0].value == "NULL"

    def test_true_uppercase(self) -> None:
        """true normalizes to KEYWORD('TRUE')."""
        tokens = non_eof("true")
        assert tokens[0].value == "TRUE"

    def test_false_uppercase(self) -> None:
        """false normalizes to KEYWORD('FALSE')."""
        tokens = non_eof("false")
        assert tokens[0].value == "FALSE"


# ---------------------------------------------------------------------------
# TestMultipleTokensInSequence
# ---------------------------------------------------------------------------


class TestMultipleTokensInSequence:
    """Verify correct tokenization when multiple tokens appear in sequence."""

    def test_comma_separated_names(self) -> None:
        """id, name, age → NAME COMMA NAME COMMA NAME."""
        types = token_type_names("id, name, age")
        assert types == ["NAME", "COMMA", "NAME", "COMMA", "NAME"]

    def test_expression_with_operators(self) -> None:
        """x + y * z → NAME PLUS NAME STAR NAME."""
        types = token_type_names("x + y * z")
        assert types == ["NAME", "PLUS", "NAME", "STAR", "NAME"]

    def test_insert_into_values(self) -> None:
        """INSERT INTO t VALUES (1, 'a') keywords."""
        tokens = non_eof("INSERT INTO t VALUES (1, 'a')")
        keywords = [t.value for t in tokens if type_name(t) == "KEYWORD"]
        assert keywords == ["INSERT", "INTO", "VALUES"]

    def test_select_star_from(self) -> None:
        """SELECT * FROM t → KEYWORD STAR KEYWORD NAME."""
        types = token_type_names("SELECT * FROM t")
        assert types == ["KEYWORD", "STAR", "KEYWORD", "NAME"]


# ---------------------------------------------------------------------------
# TestEOF
# ---------------------------------------------------------------------------


class TestEOF:
    """Tests for the EOF sentinel token."""

    def test_always_ends_with_eof(self) -> None:
        """Token list always ends with EOF."""
        tokens = tokenize_sql("SELECT 1")
        last = tokens[-1]
        assert (last.type if isinstance(last.type, str) else last.type.name) == "EOF"

    def test_empty_input_has_eof(self) -> None:
        """Empty input still produces an EOF token."""
        tokens = tokenize_sql("")
        assert len(tokens) == 1
        assert (tokens[0].type if isinstance(tokens[0].type, str) else tokens[0].type.name) == "EOF"
