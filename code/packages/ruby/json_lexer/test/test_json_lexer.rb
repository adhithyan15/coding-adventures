# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the JSON Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with json.tokens, correctly tokenizes JSON text (RFC 8259).
#
# JSON is the simplest practical language for the grammar-driven
# lexer. It has no keywords, no indentation mode, no reserved
# words, and no comments. The entire token vocabulary is:
#
#   Value tokens:     STRING, NUMBER, TRUE, FALSE, NULL
#   Structure tokens: LBRACE, RBRACE, LBRACKET, RBRACKET, COLON, COMMA
#
# Whitespace (spaces, tabs, carriage returns, newlines) is silently
# consumed -- no NEWLINE or INDENT/DEDENT tokens are emitted.
#
# We are not testing the lexer engine itself (that is tested in the
# lexer gem) -- we are testing that the JSON token grammar file
# correctly describes JSON's lexical rules.
# ================================================================

class TestJsonLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  # ---------------------------------------------------------------
  # JSON defines token types beyond the base TokenType enum.
  # The GrammarLexer uses raw string names for types not in
  # TokenType::ALL. These constants make tests readable.
  # ---------------------------------------------------------------
  NUMBER_TYPE = "NUMBER"
  TRUE_TYPE   = "TRUE"
  FALSE_TYPE  = "FALSE"
  NULL_TYPE   = "NULL"

  # ------------------------------------------------------------------
  # Helper: tokenize source and provide convenient accessors
  # ------------------------------------------------------------------

  def tokenize(source)
    CodingAdventures::JsonLexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Numbers: integers
  # ------------------------------------------------------------------
  # JSON numbers are a single token that includes an optional leading
  # minus sign. The NUMBER regex is:
  #   /-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?/
  #
  # This means -42 is ONE token, not MINUS followed by NUMBER.

  def test_integer
    tokens = tokenize("42")
    assert_equal NUMBER_TYPE, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_zero
    tokens = tokenize("0")
    assert_equal NUMBER_TYPE, tokens[0].type
    assert_equal "0", tokens[0].value
  end

  def test_negative_integer
    tokens = tokenize("-42")
    assert_equal NUMBER_TYPE, tokens[0].type
    assert_equal "-42", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Numbers: decimals
  # ------------------------------------------------------------------

  def test_decimal
    tokens = tokenize("3.14")
    assert_equal NUMBER_TYPE, tokens[0].type
    assert_equal "3.14", tokens[0].value
  end

  def test_negative_decimal
    tokens = tokenize("-0.5")
    assert_equal NUMBER_TYPE, tokens[0].type
    assert_equal "-0.5", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Numbers: exponent notation
  # ------------------------------------------------------------------
  # JSON supports scientific notation: 1e10, 1E10, 1e+10, 1e-10,
  # 2.5e3, etc. The exponent is part of the NUMBER token.

  def test_exponent
    tokens = tokenize("1e10")
    assert_equal NUMBER_TYPE, tokens[0].type
    assert_equal "1e10", tokens[0].value
  end

  def test_exponent_uppercase
    tokens = tokenize("1E10")
    assert_equal NUMBER_TYPE, tokens[0].type
    assert_equal "1E10", tokens[0].value
  end

  def test_exponent_with_sign
    tokens = tokenize("2.5e-3")
    assert_equal NUMBER_TYPE, tokens[0].type
    assert_equal "2.5e-3", tokens[0].value
  end

  def test_negative_exponent
    tokens = tokenize("-1e+10")
    assert_equal NUMBER_TYPE, tokens[0].type
    assert_equal "-1e+10", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Strings: basic string literals
  # ------------------------------------------------------------------
  # JSON strings are always double-quoted. The lexer strips the
  # surrounding quotes and returns the inner content as the value.

  def test_simple_string
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_empty_string
    tokens = tokenize('""')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "", tokens[0].value
  end

  def test_string_with_spaces
    tokens = tokenize('"Hello, World!"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "Hello, World!", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Strings: escape sequences
  # ------------------------------------------------------------------
  # JSON supports these escape sequences: \" \\ \/ \b \f \n \r \t
  # and \uXXXX for unicode code points.

  def test_string_with_escapes
    tokens = tokenize('"line1\\nline2"')
    assert_equal TT::STRING, tokens[0].type
    # The lexer processes escape sequences, so \n becomes a real newline
    assert_includes tokens[0].value, "line1"
    assert_includes tokens[0].value, "line2"
  end

  def test_string_with_escaped_quote
    tokens = tokenize('"say \\"hi\\""')
    assert_equal TT::STRING, tokens[0].type
  end

  def test_string_with_unicode_escape
    tokens = tokenize('"\\u0041"')
    assert_equal TT::STRING, tokens[0].type
  end

  # ------------------------------------------------------------------
  # Value literals: true, false, null
  # ------------------------------------------------------------------
  # In JSON, true/false/null are NOT keywords reclassified from NAME
  # (JSON has no NAME token). They are their own distinct token types:
  # TRUE, FALSE, NULL respectively.

  def test_true
    tokens = tokenize("true")
    assert_equal TRUE_TYPE, tokens[0].type
    assert_equal "true", tokens[0].value
  end

  def test_false
    tokens = tokenize("false")
    assert_equal FALSE_TYPE, tokens[0].type
    assert_equal "false", tokens[0].value
  end

  def test_null
    tokens = tokenize("null")
    assert_equal NULL_TYPE, tokens[0].type
    assert_equal "null", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Structural tokens: braces, brackets, colon, comma
  # ------------------------------------------------------------------
  # JSON uses six structural characters:
  #   { } [ ] : ,
  # These are the scaffolding that organizes values into objects
  # and arrays.

  def test_lbrace
    tokens = tokenize("{")
    assert_equal TT::LBRACE, tokens[0].type
    assert_equal "{", tokens[0].value
  end

  def test_rbrace
    tokens = tokenize("}")
    assert_equal TT::RBRACE, tokens[0].type
    assert_equal "}", tokens[0].value
  end

  def test_lbracket
    tokens = tokenize("[")
    assert_equal TT::LBRACKET, tokens[0].type
    assert_equal "[", tokens[0].value
  end

  def test_rbracket
    tokens = tokenize("]")
    assert_equal TT::RBRACKET, tokens[0].type
    assert_equal "]", tokens[0].value
  end

  def test_colon
    tokens = tokenize(":")
    assert_equal TT::COLON, tokens[0].type
    assert_equal ":", tokens[0].value
  end

  def test_comma
    tokens = tokenize(",")
    assert_equal TT::COMMA, tokens[0].type
    assert_equal ",", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Whitespace handling
  # ------------------------------------------------------------------
  # JSON whitespace (space, tab, CR, LF) is insignificant between
  # tokens. The lexer should skip it silently -- no NEWLINE, INDENT,
  # or DEDENT tokens should appear.

  def test_whitespace_is_skipped
    tokens = tokenize("  42  ")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal NUMBER_TYPE, non_eof[0].type
    assert_equal "42", non_eof[0].value
  end

  def test_newline_after_value
    # The JSON grammar's skip pattern includes \n (WHITESPACE = /[ \t\r\n]+/),
    # which consumes newlines silently. No NEWLINE tokens are emitted — this
    # is correct for JSON where newlines are just whitespace.
    tokens = tokenize("42\n")
    types = tokens.map(&:type)
    assert_equal [NUMBER_TYPE, TT::EOF], types
  end

  def test_tabs_and_newlines_in_json
    # The JSON skip pattern consumes tabs, spaces, and newlines. Only
    # structural tokens remain in the output.
    tokens = tokenize("{\n\t\"key\"\n}")
    types = tokens.map(&:type)
    expected = [TT::LBRACE, TT::STRING, TT::RBRACE, TT::EOF]
    assert_equal expected, types
  end

  # ------------------------------------------------------------------
  # Empty object: {}
  # ------------------------------------------------------------------

  def test_empty_object
    tokens = tokenize("{}")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACE, TT::RBRACE, TT::EOF], types
  end

  # ------------------------------------------------------------------
  # Empty array: []
  # ------------------------------------------------------------------

  def test_empty_array
    tokens = tokenize("[]")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACKET, TT::RBRACKET, TT::EOF], types
  end

  # ------------------------------------------------------------------
  # Complete JSON object
  # ------------------------------------------------------------------
  # Tokenize a realistic JSON object to verify that all token types
  # work together correctly in a complete document.

  def test_complete_json_object
    source = '{"name": "Alice", "age": 30, "active": true}'
    tokens = tokenize(source)
    types = tokens.map(&:type)

    expected = [
      TT::LBRACE,
      TT::STRING, TT::COLON, TT::STRING, TT::COMMA,  # "name": "Alice",
      TT::STRING, TT::COLON, NUMBER_TYPE, TT::COMMA,  # "age": 30,
      TT::STRING, TT::COLON, TRUE_TYPE,                # "active": true
      TT::RBRACE,
      TT::EOF
    ]
    assert_equal expected, types
  end

  def test_complete_json_object_values
    source = '{"name": "Alice", "age": 30, "active": true}'
    tokens = tokenize(source)
    values = tokens.map(&:value)

    assert_equal "{", values[0]
    assert_equal "name", values[1]      # STRING key (quotes stripped)
    assert_equal ":", values[2]
    assert_equal "Alice", values[3]     # STRING value (quotes stripped)
    assert_equal ",", values[4]
    assert_equal "age", values[5]
    assert_equal ":", values[6]
    assert_equal "30", values[7]
    assert_equal ",", values[8]
    assert_equal "active", values[9]
    assert_equal ":", values[10]
    assert_equal "true", values[11]
    assert_equal "}", values[12]
  end

  # ------------------------------------------------------------------
  # JSON array with mixed types
  # ------------------------------------------------------------------

  def test_array_with_mixed_types
    source = '[1, "two", true, false, null]'
    tokens = tokenize(source)
    types = tokens.map(&:type)

    expected = [
      TT::LBRACKET,
      NUMBER_TYPE, TT::COMMA,
      TT::STRING, TT::COMMA,
      TRUE_TYPE, TT::COMMA,
      FALSE_TYPE, TT::COMMA,
      NULL_TYPE,
      TT::RBRACKET,
      TT::EOF
    ]
    assert_equal expected, types
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::JsonLexer::JSON_TOKENS_PATH),
      "json.tokens file should exist at #{CodingAdventures::JsonLexer::JSON_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Line tracking
  # ------------------------------------------------------------------

  def test_line_tracking
    source = "{\n  \"key\": 42\n}"
    tokens = tokenize(source)
    # Find the "key" string token -- it should be on line 2
    key_token = tokens.find { |t| t.type == TT::STRING && t.value == "key" }
    refute_nil key_token, "Expected STRING token 'key'"
    assert_equal 2, key_token.line, "Expected 'key' to be on line 2"
  end

  # ------------------------------------------------------------------
  # EOF token
  # ------------------------------------------------------------------
  # Every token stream should end with an EOF token.

  def test_eof_token
    tokens = tokenize("42")
    assert_equal TT::EOF, tokens.last.type
  end

  def test_empty_input_eof
    tokens = tokenize("")
    assert_equal TT::EOF, tokens.last.type
  end
end
