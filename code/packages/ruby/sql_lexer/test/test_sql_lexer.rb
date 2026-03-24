# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the SQL Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with sql.tokens, correctly tokenizes ANSI SQL text.
#
# SQL differs from JSON in several important ways:
#
# 1. **Keywords**: SQL has ~50 reserved keywords (SELECT, FROM, etc.)
#    that are recognized from the NAME token type. Keywords have type
#    "KEYWORD" and their value is normalized to uppercase because
#    sql.tokens uses @case_insensitive true.
#
# 2. **Case insensitivity**: "select", "SELECT", and "Select" all
#    produce Token(type: KEYWORD, value: "SELECT").
#
# 3. **Comments**: SQL supports line comments (-- to end of line)
#    and block comments (/* ... */). Both are silently skipped.
#
# 4. **Single-quoted strings**: 'hello' produces Token(STRING, "hello")
#    with quotes stripped. (SQL uses single quotes, not double quotes.)
#
# 5. **Operator aliases**: "<>" and "!=" both produce NOT_EQUALS tokens.
#    "<=" and ">=" produce LESS_EQUALS and GREATER_EQUALS respectively.
#
# 6. **Backtick identifiers**: `foo` produces a NAME token but the
#    backticks are NOT stripped -- the value is "`foo`".
#
# Token type reference:
#
#   TokenType constants (from TokenType::ALL):
#     TT::NAME, TT::NUMBER, TT::STRING, TT::KEYWORD,
#     TT::EQUALS, TT::PLUS, TT::MINUS, TT::STAR, TT::SLASH,
#     TT::LPAREN, TT::RPAREN, TT::COMMA, TT::SEMICOLON, TT::DOT, TT::EOF
#
#   Grammar-specific strings (not in TokenType::ALL):
#     "NOT_EQUALS", "LESS_THAN", "GREATER_THAN",
#     "LESS_EQUALS", "GREATER_EQUALS", "PERCENT"
#
# We are not testing the lexer engine (that is tested in the lexer gem).
# We are testing that the SQL token grammar file correctly describes
# SQL's lexical rules.
# ================================================================

class TestSqlLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  # SQL-specific operator types (strings, not in TokenType::ALL)
  NOT_EQUALS_TYPE    = "NOT_EQUALS"
  LESS_THAN_TYPE     = "LESS_THAN"
  GREATER_THAN_TYPE  = "GREATER_THAN"
  LESS_EQUALS_TYPE   = "LESS_EQUALS"
  GREATER_EQUALS_TYPE = "GREATER_EQUALS"
  PERCENT_TYPE       = "PERCENT"

  # ------------------------------------------------------------------
  # Helpers
  # ------------------------------------------------------------------

  def tokenize(source)
    CodingAdventures::SqlLexer.tokenize_sql(source)
  end

  def non_eof(source)
    tokenize(source).reject { |t| t.type == TT::EOF }
  end

  # ------------------------------------------------------------------
  # create_sql_lexer
  # ------------------------------------------------------------------
  # The create_sql_lexer method should return a configured lexer object.
  # This is useful for users who want a lexer reference before calling
  # tokenize (e.g., to register additional callbacks).

  def test_create_sql_lexer_returns_non_nil
    lexer = CodingAdventures::SqlLexer.create_sql_lexer("SELECT 1")
    refute_nil lexer, "create_sql_lexer should return a lexer object"
  end

  def test_create_sql_lexer_returns_grammar_lexer
    lexer = CodingAdventures::SqlLexer.create_sql_lexer("SELECT 1")
    assert_instance_of CodingAdventures::Lexer::GrammarLexer, lexer
  end

  def test_tokenize_sql_returns_tokens
    tokens = CodingAdventures::SqlLexer.tokenize_sql("SELECT 1")
    assert_instance_of Array, tokens
    refute_empty tokens
  end

  # ------------------------------------------------------------------
  # Grammar path
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::SqlLexer::SQL_TOKENS_PATH),
      "sql.tokens file should exist at #{CodingAdventures::SqlLexer::SQL_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Error path: invalid grammar
  # ------------------------------------------------------------------
  # If create_sql_lexer receives a path that doesn't exist or contains
  # bad content, it should raise an error.

  def test_error_with_nonexistent_grammar
    # Temporarily override the constant to point to a nonexistent file
    # by directly testing File.read behavior
    assert_raises(StandardError) do
      CodingAdventures::GrammarTools.parse_token_grammar(
        File.read("/nonexistent/path/to/sql.tokens", encoding: "UTF-8")
      )
    end
  end

  def test_error_with_bad_grammar_content
    # A temp file with invalid grammar content should raise
    require "tempfile"
    Tempfile.open(["bad_grammar", ".tokens"]) do |f|
      f.write("!!!invalid grammar content!!!")
      f.flush
      assert_raises(StandardError) do
        CodingAdventures::GrammarTools.parse_token_grammar(
          File.read(f.path, encoding: "UTF-8")
        )
      end
    end
  end

  # ------------------------------------------------------------------
  # EOF token
  # ------------------------------------------------------------------

  def test_eof_token
    tokens = tokenize("SELECT 1")
    assert_equal TT::EOF, tokens.last.type
  end

  def test_empty_input_produces_eof
    tokens = tokenize("")
    assert_equal 1, tokens.length
    assert_equal TT::EOF, tokens.last.type
  end

  # ------------------------------------------------------------------
  # Keywords: basic
  # ------------------------------------------------------------------
  # SQL keywords are matched from the NAME pattern by comparing against
  # the keywords list. Because @case_insensitive is true, both the
  # keyword set and the matched value are uppercased before comparing.
  # The emitted token has type KEYWORD and value normalized to uppercase.

  def test_select_keyword
    tokens = non_eof("SELECT")
    assert_equal 1, tokens.length
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "SELECT", tokens[0].value
  end

  def test_from_keyword
    tokens = non_eof("FROM")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "FROM", tokens[0].value
  end

  def test_where_keyword
    tokens = non_eof("WHERE")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "WHERE", tokens[0].value
  end

  def test_insert_keyword
    tokens = non_eof("INSERT")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "INSERT", tokens[0].value
  end

  def test_update_keyword
    tokens = non_eof("UPDATE")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "UPDATE", tokens[0].value
  end

  def test_delete_keyword
    tokens = non_eof("DELETE")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "DELETE", tokens[0].value
  end

  def test_create_keyword
    tokens = non_eof("CREATE")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "CREATE", tokens[0].value
  end

  def test_drop_keyword
    tokens = non_eof("DROP")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "DROP", tokens[0].value
  end

  def test_table_keyword
    tokens = non_eof("TABLE")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "TABLE", tokens[0].value
  end

  def test_null_keyword
    tokens = non_eof("NULL")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "NULL", tokens[0].value
  end

  def test_true_keyword
    tokens = non_eof("TRUE")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "TRUE", tokens[0].value
  end

  def test_false_keyword
    tokens = non_eof("FALSE")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "FALSE", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Keywords: case insensitivity
  # ------------------------------------------------------------------
  # Because sql.tokens has @case_insensitive true, the lexer normalizes
  # keyword values to uppercase. "select", "SELECT", and "Select" all
  # produce Token(type: KEYWORD, value: "SELECT").

  def test_keyword_lowercase_select
    tokens = non_eof("select")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "SELECT", tokens[0].value,
      "lowercase 'select' should be normalized to 'SELECT'"
  end

  def test_keyword_uppercase_select
    tokens = non_eof("SELECT")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "SELECT", tokens[0].value
  end

  def test_keyword_mixed_case_select
    tokens = non_eof("Select")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "SELECT", tokens[0].value,
      "mixed-case 'Select' should be normalized to 'SELECT'"
  end

  def test_keyword_lowercase_from
    tokens = non_eof("from")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "FROM", tokens[0].value
  end

  def test_keyword_mixed_case_where
    tokens = non_eof("Where")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "WHERE", tokens[0].value
  end

  def test_keyword_all_caps_insert
    tokens = non_eof("INSERT")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "INSERT", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Identifiers: NAME
  # ------------------------------------------------------------------
  # Unquoted identifiers that are NOT keywords produce NAME tokens
  # with their original casing preserved.

  def test_simple_identifier
    tokens = non_eof("users")
    assert_equal 1, tokens.length
    assert_equal TT::NAME, tokens[0].type
    assert_equal "users", tokens[0].value
  end

  def test_identifier_with_underscore
    tokens = non_eof("order_id")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "order_id", tokens[0].value
  end

  def test_identifier_with_numbers
    tokens = non_eof("col1")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "col1", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Identifiers: QUOTED_ID (backtick-quoted)
  # ------------------------------------------------------------------
  # Backtick-quoted identifiers are aliased to NAME in the grammar:
  #   QUOTED_ID = /`[^`]+`/ -> NAME
  # The backticks are NOT stripped (unlike single-quoted strings).

  def test_backtick_identifier_type_is_name
    tokens = non_eof("`orders`")
    assert_equal 1, tokens.length
    assert_equal TT::NAME, tokens[0].type
  end

  def test_backtick_identifier_value_keeps_backticks
    tokens = non_eof("`orders`")
    assert_equal "`orders`", tokens[0].value,
      "backtick-quoted identifiers should retain their backticks"
  end

  def test_backtick_identifier_with_spaces
    tokens = non_eof("`my table`")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "`my table`", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Numbers: integers
  # ------------------------------------------------------------------

  def test_integer
    tokens = non_eof("42")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_zero
    tokens = non_eof("0")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "0", tokens[0].value
  end

  def test_large_integer
    tokens = non_eof("1000000")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "1000000", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Numbers: decimals
  # ------------------------------------------------------------------

  def test_decimal_number
    tokens = non_eof("3.14")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "3.14", tokens[0].value
  end

  def test_decimal_zero_point_five
    tokens = non_eof("0.5")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "0.5", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Strings: single-quoted
  # ------------------------------------------------------------------
  # SQL uses single quotes for string literals:
  #   'hello' -> Token(STRING, "hello")  (quotes stripped)

  def test_simple_string
    tokens = non_eof("'hello'")
    assert_equal 1, tokens.length
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_empty_string
    tokens = non_eof("''")
    assert_equal TT::STRING, tokens[0].type
    assert_equal "", tokens[0].value
  end

  def test_string_with_spaces
    tokens = non_eof("'hello world'")
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello world", tokens[0].value
  end

  def test_string_with_escape
    tokens = non_eof("'it\\'s'")
    assert_equal TT::STRING, tokens[0].type
  end

  # ------------------------------------------------------------------
  # Operators: basic
  # ------------------------------------------------------------------

  def test_equals_operator
    tokens = non_eof("=")
    assert_equal TT::EQUALS, tokens[0].type
    assert_equal "=", tokens[0].value
  end

  def test_plus_operator
    tokens = non_eof("+")
    assert_equal TT::PLUS, tokens[0].type
    assert_equal "+", tokens[0].value
  end

  def test_minus_operator
    tokens = non_eof("-")
    assert_equal TT::MINUS, tokens[0].type
    assert_equal "-", tokens[0].value
  end

  def test_star_operator
    tokens = non_eof("*")
    assert_equal TT::STAR, tokens[0].type
    assert_equal "*", tokens[0].value
  end

  def test_slash_operator
    tokens = non_eof("/")
    assert_equal TT::SLASH, tokens[0].type
    assert_equal "/", tokens[0].value
  end

  def test_percent_operator
    tokens = non_eof("%")
    assert_equal PERCENT_TYPE, tokens[0].type
    assert_equal "%", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Operators: comparison
  # ------------------------------------------------------------------
  # Longest match first ensures "<=" is matched before "<", and ">="
  # before ">", and "!=" before nothing. "<>" is aliased to NOT_EQUALS.

  def test_less_than
    tokens = non_eof("<")
    assert_equal LESS_THAN_TYPE, tokens[0].type
    assert_equal "<", tokens[0].value
  end

  def test_greater_than
    tokens = non_eof(">")
    assert_equal GREATER_THAN_TYPE, tokens[0].type
    assert_equal ">", tokens[0].value
  end

  def test_not_equals_bang
    tokens = non_eof("!=")
    assert_equal NOT_EQUALS_TYPE, tokens[0].type
    assert_equal "!=", tokens[0].value
  end

  def test_not_equals_ansi
    # SQL standard uses <> for inequality; sql.tokens maps NEQ_ANSI -> NOT_EQUALS
    tokens = non_eof("<>")
    assert_equal NOT_EQUALS_TYPE, tokens[0].type
    assert_equal "<>", tokens[0].value
  end

  def test_less_equals
    tokens = non_eof("<=")
    assert_equal LESS_EQUALS_TYPE, tokens[0].type
    assert_equal "<=", tokens[0].value
  end

  def test_greater_equals
    tokens = non_eof(">=")
    assert_equal GREATER_EQUALS_TYPE, tokens[0].type
    assert_equal ">=", tokens[0].value
  end

  # Verify longest-match: "<=" is preferred over "<" followed by "="
  def test_less_equals_is_single_token_not_two
    tokens = non_eof("<=")
    assert_equal 1, tokens.length, "< = should be ONE token (LESS_EQUALS)"
  end

  def test_greater_equals_is_single_token_not_two
    tokens = non_eof(">=")
    assert_equal 1, tokens.length, "> = should be ONE token (GREATER_EQUALS)"
  end

  # ------------------------------------------------------------------
  # Punctuation
  # ------------------------------------------------------------------

  def test_lparen
    tokens = non_eof("(")
    assert_equal TT::LPAREN, tokens[0].type
    assert_equal "(", tokens[0].value
  end

  def test_rparen
    tokens = non_eof(")")
    assert_equal TT::RPAREN, tokens[0].type
    assert_equal ")", tokens[0].value
  end

  def test_comma
    tokens = non_eof(",")
    assert_equal TT::COMMA, tokens[0].type
    assert_equal ",", tokens[0].value
  end

  def test_semicolon
    tokens = non_eof(";")
    assert_equal TT::SEMICOLON, tokens[0].type
    assert_equal ";", tokens[0].value
  end

  def test_dot
    tokens = non_eof(".")
    assert_equal TT::DOT, tokens[0].type
    assert_equal ".", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Comment skipping
  # ------------------------------------------------------------------
  # SQL line comments start with "--" and run to end of line.
  # SQL block comments are /* ... */ and may span multiple lines.
  # Both are silently skipped -- they do not appear in the token stream.

  def test_line_comment_skipped
    # The comment should be consumed, leaving only the NUMBER token.
    tokens = non_eof("42 -- this is a comment\n")
    assert_equal 1, tokens.length, "line comment should be skipped"
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_line_comment_only
    tokens = non_eof("-- just a comment")
    assert_empty tokens, "a line comment alone should produce no tokens"
  end

  def test_block_comment_skipped
    tokens = non_eof("42 /* block comment */ 99")
    assert_equal 2, tokens.length, "block comment should be skipped"
    assert_equal "42", tokens[0].value
    assert_equal "99", tokens[1].value
  end

  def test_block_comment_only
    tokens = non_eof("/* this is a block comment */")
    assert_empty tokens, "a block comment alone should produce no tokens"
  end

  def test_block_comment_multiline
    source = "SELECT /* multi\nline\ncomment */ 1"
    tokens = non_eof(source)
    # Should have: SELECT, 1
    assert_equal 2, tokens.length
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "SELECT", tokens[0].value
    assert_equal TT::NUMBER, tokens[1].type
  end

  # ------------------------------------------------------------------
  # Whitespace skipping
  # ------------------------------------------------------------------
  # Spaces, tabs, carriage returns, and newlines are all silently consumed.

  def test_whitespace_skipped
    tokens = non_eof("  42  ")
    assert_equal 1, tokens.length
    assert_equal "42", tokens[0].value
  end

  def test_newlines_skipped
    tokens = non_eof("42\n99")
    assert_equal 2, tokens.length
    assert_equal "42", tokens[0].value
    assert_equal "99", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Qualified names (schema.table notation)
  # ------------------------------------------------------------------
  # schema.orders -> NAME DOT NAME
  # The DOT is a punctuation token, not part of the identifier.

  def test_qualified_name
    tokens = non_eof("schema.orders")
    assert_equal 3, tokens.length
    assert_equal TT::NAME, tokens[0].type
    assert_equal "schema", tokens[0].value
    assert_equal TT::DOT, tokens[1].type
    assert_equal ".", tokens[1].value
    assert_equal TT::NAME, tokens[2].type
    assert_equal "orders", tokens[2].value
  end

  # ------------------------------------------------------------------
  # Complete SQL statements
  # ------------------------------------------------------------------
  # Test that realistic SQL tokenizes correctly end-to-end.

  def test_simple_select
    # SELECT id FROM users
    tokens = non_eof("SELECT id FROM users")
    types = tokens.map(&:type)
    values = tokens.map(&:value)

    assert_equal [TT::KEYWORD, TT::NAME, TT::KEYWORD, TT::NAME], types
    assert_equal ["SELECT", "id", "FROM", "users"], values
  end

  def test_select_with_where
    # SELECT * FROM orders WHERE id = 1
    tokens = non_eof("SELECT * FROM orders WHERE id = 1")
    values = tokens.map(&:value)

    assert_includes values, "SELECT"
    assert_includes values, "*"
    assert_includes values, "FROM"
    assert_includes values, "WHERE"
    assert_includes values, "="
    assert_includes values, "1"
  end

  def test_insert_statement
    # INSERT INTO users (name) VALUES ('Alice')
    tokens = non_eof("INSERT INTO users (name) VALUES ('Alice')")
    types = tokens.map(&:type)
    values = tokens.map(&:value)

    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "INSERT", tokens[0].value
    assert_includes values, "INTO"
    assert_includes values, "Alice"
    assert_includes types, TT::STRING
  end

  def test_update_statement
    # UPDATE users SET name = 'Bob' WHERE id = 1
    tokens = non_eof("UPDATE users SET name = 'Bob' WHERE id = 1")
    values = tokens.map(&:value)

    assert_equal "UPDATE", values[0]
    assert_includes values, "SET"
    assert_includes values, "Bob"
  end

  def test_delete_statement
    # DELETE FROM orders WHERE id = 42
    tokens = non_eof("DELETE FROM orders WHERE id = 42")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "DELETE", tokens[0].value
  end

  def test_select_star
    tokens = non_eof("SELECT * FROM orders")
    star_token = tokens.find { |t| t.type == TT::STAR }
    refute_nil star_token, "Expected STAR token for *"
    assert_equal "*", star_token.value
  end

  def test_select_with_comparison_operators
    tokens = non_eof("SELECT a FROM t WHERE b >= 5 AND c != 3")
    types = tokens.map(&:type)

    assert_includes types, GREATER_EQUALS_TYPE
    assert_includes types, NOT_EQUALS_TYPE
  end

  def test_create_table_statement
    sql = "CREATE TABLE users (id NUMBER, name NAME)"
    tokens = non_eof(sql)
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "CREATE", tokens[0].value
    assert_equal TT::KEYWORD, tokens[1].type
    assert_equal "TABLE", tokens[1].value
  end

  def test_drop_table_statement
    tokens = non_eof("DROP TABLE orders")
    assert_equal "DROP", tokens[0].value
    assert_equal "TABLE", tokens[1].value
  end

  def test_case_insensitive_in_full_select
    # All lowercase keywords should be normalized
    tokens = non_eof("select id from users where active = 1")
    kw_tokens = tokens.select { |t| t.type == TT::KEYWORD }
    kw_values = kw_tokens.map(&:value)

    assert_equal ["SELECT", "FROM", "WHERE"], kw_values,
      "all keywords should be normalized to uppercase"
  end

  def test_select_with_semicolon
    tokens = non_eof("SELECT 1;")
    last_non_eof = tokens.last
    assert_equal TT::SEMICOLON, last_non_eof.type
    assert_equal ";", last_non_eof.value
  end

  def test_multiple_statements_separated_by_semicolons
    tokens = non_eof("SELECT 1; SELECT 2")
    semicolons = tokens.select { |t| t.type == TT::SEMICOLON }
    assert_equal 1, semicolons.length
  end

  # ------------------------------------------------------------------
  # Line tracking
  # ------------------------------------------------------------------

  def test_line_tracking_first_line
    tokens = tokenize("SELECT")
    kw = tokens.find { |t| t.type == TT::KEYWORD }
    assert_equal 1, kw.line
  end

  def test_line_tracking_second_line
    tokens = tokenize("SELECT\nFROM")
    from_token = tokens.find { |t| t.value == "FROM" }
    refute_nil from_token
    assert_equal 2, from_token.line
  end
end
