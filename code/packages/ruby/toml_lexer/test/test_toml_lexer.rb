# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the TOML Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded with
# toml.tokens, correctly tokenizes TOML v1.0.0 text.
#
# TOML has significantly more token types than JSON (20 vs 11) and
# introduces several challenges:
#
#   1. Newline sensitivity -- NEWLINE tokens are emitted (not skipped)
#   2. Token ordering -- first-match-wins must be correct
#   3. Multiple string types -- 4 kinds with different rules
#   4. Date/time literals -- must match before bare keys and integers
#   5. Number variants -- hex, octal, binary, special floats
#   6. Bare keys -- must be tried last (matches everything)
#
# Note: The GrammarLexer returns all token types as strings because
# they are defined in the .tokens file, not the base TokenType enum.
# This includes delimiters like "LBRACKET" and structural tokens like
# "NEWLINE" and "EOF".
# ================================================================

class TestTomlLexer < Minitest::Test
  def tokenize(source)
    CodingAdventures::TomlLexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map { |t| t.type.to_s }
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Basic key-value pairs
  # ------------------------------------------------------------------

  def test_bare_key_equals_string
    types = token_types('name = "TOML"')
    assert_includes types, "BARE_KEY"
    assert_includes types, "BASIC_STRING"
    assert_includes types, "EOF"
  end

  def test_bare_key_equals_integer
    values = token_values("port = 8080")
    assert_includes values, "port"
    assert_includes values, "8080"
  end

  # ------------------------------------------------------------------
  # String types
  # ------------------------------------------------------------------

  def test_basic_string
    tokens = tokenize('key = "hello"')
    string_token = tokens.find { |t| t.type.to_s == "BASIC_STRING" }
    refute_nil string_token
    # escapes: none means quotes are preserved in the raw token
    assert_equal '"hello"', string_token.value
  end

  def test_literal_string
    tokens = tokenize("key = 'hello'")
    string_token = tokens.find { |t| t.type.to_s == "LITERAL_STRING" }
    refute_nil string_token
    assert_equal "'hello'", string_token.value
  end

  def test_ml_basic_string
    tokens = tokenize("key = \"\"\"hello\nworld\"\"\"")
    string_token = tokens.find { |t| t.type.to_s == "ML_BASIC_STRING" }
    refute_nil string_token
  end

  def test_ml_literal_string
    tokens = tokenize("key = '''hello\nworld'''")
    string_token = tokens.find { |t| t.type.to_s == "ML_LITERAL_STRING" }
    refute_nil string_token
  end

  def test_ml_string_before_single_line
    # """hello""" must match as ML_BASIC_STRING, not three BASIC_STRINGs
    types = token_types('key = """hello"""')
    assert_includes types, "ML_BASIC_STRING"
    refute_includes types, "BASIC_STRING"
  end

  # ------------------------------------------------------------------
  # Number types
  # ------------------------------------------------------------------

  def test_integer
    tokens = tokenize("key = 42")
    int_token = tokens.find { |t| t.type.to_s == "INTEGER" }
    refute_nil int_token
    assert_equal "42", int_token.value
  end

  def test_negative_integer
    tokens = tokenize("key = -17")
    int_token = tokens.find { |t| t.type.to_s == "INTEGER" }
    refute_nil int_token
    assert_equal "-17", int_token.value
  end

  def test_hex_integer
    tokens = tokenize("key = 0xFF")
    int_token = tokens.find { |t| t.type.to_s == "INTEGER" }
    refute_nil int_token
    assert_equal "0xFF", int_token.value
  end

  def test_octal_integer
    tokens = tokenize("key = 0o77")
    int_token = tokens.find { |t| t.type.to_s == "INTEGER" }
    refute_nil int_token
    assert_equal "0o77", int_token.value
  end

  def test_binary_integer
    tokens = tokenize("key = 0b1010")
    int_token = tokens.find { |t| t.type.to_s == "INTEGER" }
    refute_nil int_token
    assert_equal "0b1010", int_token.value
  end

  def test_underscore_integer
    tokens = tokenize("key = 1_000_000")
    int_token = tokens.find { |t| t.type.to_s == "INTEGER" }
    refute_nil int_token
    assert_equal "1_000_000", int_token.value
  end

  def test_float_decimal
    tokens = tokenize("key = 3.14")
    float_token = tokens.find { |t| t.type.to_s == "FLOAT" }
    refute_nil float_token
    assert_equal "3.14", float_token.value
  end

  def test_float_scientific
    tokens = tokenize("key = 1e10")
    float_token = tokens.find { |t| t.type.to_s == "FLOAT" }
    refute_nil float_token
  end

  def test_float_inf
    tokens = tokenize("key = inf")
    float_token = tokens.find { |t| t.type.to_s == "FLOAT" }
    refute_nil float_token
    assert_equal "inf", float_token.value
  end

  def test_float_nan
    tokens = tokenize("key = nan")
    float_token = tokens.find { |t| t.type.to_s == "FLOAT" }
    refute_nil float_token
  end

  # ------------------------------------------------------------------
  # Boolean types
  # ------------------------------------------------------------------

  def test_true
    tokens = tokenize("key = true")
    true_token = tokens.find { |t| t.type.to_s == "TRUE" }
    refute_nil true_token
  end

  def test_false
    tokens = tokenize("key = false")
    false_token = tokens.find { |t| t.type.to_s == "FALSE" }
    refute_nil false_token
  end

  # ------------------------------------------------------------------
  # Date/time types
  # ------------------------------------------------------------------

  def test_offset_datetime
    tokens = tokenize("key = 1979-05-27T07:32:00Z")
    dt_token = tokens.find { |t| t.type.to_s == "OFFSET_DATETIME" }
    refute_nil dt_token
    assert_equal "1979-05-27T07:32:00Z", dt_token.value
  end

  def test_local_datetime
    tokens = tokenize("key = 1979-05-27T07:32:00")
    dt_token = tokens.find { |t| t.type.to_s == "LOCAL_DATETIME" }
    refute_nil dt_token
  end

  def test_local_date
    tokens = tokenize("key = 1979-05-27")
    dt_token = tokens.find { |t| t.type.to_s == "LOCAL_DATE" }
    refute_nil dt_token
    assert_equal "1979-05-27", dt_token.value
  end

  def test_local_time
    tokens = tokenize("key = 07:32:00")
    dt_token = tokens.find { |t| t.type.to_s == "LOCAL_TIME" }
    refute_nil dt_token
  end

  def test_date_not_bare_key
    # 1979-05-27 must match as LOCAL_DATE, not BARE_KEY
    types = token_types("key = 1979-05-27")
    assert types.count("BARE_KEY") <= 1, "Date should not split into bare keys"
  end

  # ------------------------------------------------------------------
  # Bare keys
  # ------------------------------------------------------------------

  def test_bare_key
    tokens = tokenize("my-key = 1")
    key_token = tokens.find { |t| t.type.to_s == "BARE_KEY" }
    refute_nil key_token
    assert_equal "my-key", key_token.value
  end

  def test_bare_key_last_priority
    # "true" should match as TRUE, not BARE_KEY
    types = token_types("key = true")
    assert_includes types, "TRUE"
    # Only the "key" part should be BARE_KEY
    bare_keys = tokenize("key = true").select { |t| t.type.to_s == "BARE_KEY" }
    assert_equal 1, bare_keys.length
    assert_equal "key", bare_keys[0].value
  end

  # ------------------------------------------------------------------
  # Delimiters
  # ------------------------------------------------------------------

  def test_delimiters
    types = token_types("[server]\nhost = {}")
    assert_includes types, "LBRACKET"
    assert_includes types, "RBRACKET"
    assert_includes types, "LBRACE"
    assert_includes types, "RBRACE"
  end

  def test_equals
    types = token_types("key = 1")
    assert_includes types, "EQUALS"
  end

  def test_dot
    types = token_types("a.b = 1")
    assert_includes types, "DOT"
  end

  def test_comma
    types = token_types("key = [1, 2]")
    assert_includes types, "COMMA"
  end

  # ------------------------------------------------------------------
  # Newline handling
  # ------------------------------------------------------------------

  def test_newline_emitted
    types = token_types("a = 1\nb = 2")
    assert_includes types, "NEWLINE"
  end

  def test_comment_skipped_but_newline_preserved
    tokens = tokenize("a = 1 # comment\nb = 2")
    types = tokens.map { |t| t.type.to_s }
    # Comment text should not appear as a token
    comment_tokens = tokens.select { |t| t.value.include?("#") }
    assert_empty comment_tokens
    # But the newline should be there
    assert_includes types, "NEWLINE"
  end

  # ------------------------------------------------------------------
  # Complex structures
  # ------------------------------------------------------------------

  def test_table_header
    types = token_types("[server]")
    assert_includes types, "LBRACKET"
    assert_includes types, "BARE_KEY"
    assert_includes types, "RBRACKET"
  end

  def test_array_of_tables_header
    tokens = tokenize("[[products]]")
    brackets = tokens.select { |t| t.type.to_s == "LBRACKET" }
    assert_equal 2, brackets.length
  end

  def test_inline_table
    types = token_types("point = { x = 1, y = 2 }")
    assert_includes types, "LBRACE"
    assert_includes types, "RBRACE"
    assert_includes types, "COMMA"
  end

  def test_array
    types = token_types("colors = [1, 2, 3]")
    assert_includes types, "LBRACKET"
    assert_includes types, "RBRACKET"
    assert_includes types, "COMMA"
  end

  # ------------------------------------------------------------------
  # Edge cases
  # ------------------------------------------------------------------

  def test_empty_string_produces_eof
    tokens = tokenize("")
    assert_equal 1, tokens.length
    assert_equal "EOF", tokens.last.type.to_s
  end

  def test_multiline_document
    source = "[server]\nhost = \"localhost\"\nport = 8080"
    tokens = tokenize(source)
    bare_keys = tokens.select { |t| t.type.to_s == "BARE_KEY" }
    assert bare_keys.length >= 3, "Expected at least 3 bare keys (server, host, port)"
  end
end
