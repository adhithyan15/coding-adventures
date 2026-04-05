# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the ECMAScript 3 (ES3) Lexer
# ================================================================
#
# ES3 (1999) added strict equality, try/catch/finally, instanceof,
# and regex literals. These tests verify ES3-specific features that
# are NOT present in ES1.
# ================================================================

class TestEcmascriptEs3Lexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  def tokenize(source)
    CodingAdventures::EcmascriptEs3Lexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Basic expression: var x = 1 + 2; (same as ES1)
  # ------------------------------------------------------------------

  def test_var_assignment
    tokens = tokenize("var x = 1 + 2;")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::NAME, TT::EQUALS, TT::NUMBER,
      TT::PLUS, TT::NUMBER, TT::SEMICOLON, TT::EOF], types
  end

  # ------------------------------------------------------------------
  # ES3-specific: strict equality === and !==
  # ------------------------------------------------------------------

  def test_strict_equals
    tokens = tokenize("x === 1")
    assert_equal "===", tokens[1].value
  end

  def test_strict_not_equals
    tokens = tokenize("x !== 1")
    assert_equal "!==", tokens[1].value
  end

  def test_abstract_equality_still_works
    tokens = tokenize("x == 1")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS_EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_abstract_not_equals_still_works
    tokens = tokenize("x != 1")
    assert_equal "!=", tokens[1].value
  end

  # ------------------------------------------------------------------
  # ES3-specific keywords: catch, finally, instanceof, throw, try
  # ------------------------------------------------------------------

  def test_keyword_try
    tokens = tokenize("try")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "try", tokens[0].value
  end

  def test_keyword_catch
    tokens = tokenize("catch")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "catch", tokens[0].value
  end

  def test_keyword_finally
    tokens = tokenize("finally")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "finally", tokens[0].value
  end

  def test_keyword_throw
    tokens = tokenize("throw")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "throw", tokens[0].value
  end

  def test_keyword_instanceof
    tokens = tokenize("instanceof")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "instanceof", tokens[0].value
  end

  # ------------------------------------------------------------------
  # ES1 keywords still work in ES3
  # ------------------------------------------------------------------

  def test_keyword_var
    tokens = tokenize("var")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "var", tokens[0].value
  end

  def test_keyword_function
    tokens = tokenize("function")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "function", tokens[0].value
  end

  def test_boolean_and_null_keywords
    tokens = tokenize("true false null")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[true false null], keywords
  end

  # ------------------------------------------------------------------
  # ES3-specific: regex literals
  # ------------------------------------------------------------------

  def test_regex_literal
    tokens = tokenize("/abc/g")
    assert_equal "REGEX", tokens[0].type
    assert_equal "/abc/g", tokens[0].value
  end

  def test_regex_with_flags
    tokens = tokenize("/test/gim")
    assert_equal "REGEX", tokens[0].type
    assert_equal "/test/gim", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Operators (inherited from ES1)
  # ------------------------------------------------------------------

  def test_compound_assignments
    tokens = tokenize("x += 1")
    assert_equal "+=", tokens[1].value
  end

  def test_shift_operators
    tokens = tokenize("a >>> 2")
    assert_equal ">>>", tokens[1].value
  end

  def test_logical_operators
    tokens = tokenize("a && b || c")
    values = tokens.map(&:value)
    assert_includes values, "&&"
    assert_includes values, "||"
  end

  def test_increment_decrement
    tokens = tokenize("x++ y--")
    assert_equal "++", tokens[1].value
    assert_equal "--", tokens[3].value
  end

  # ------------------------------------------------------------------
  # Delimiters
  # ------------------------------------------------------------------

  def test_curly_braces
    tokens = tokenize("{ }")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACE, TT::RBRACE, TT::EOF], types
  end

  def test_semicolon
    tokens = tokenize(";")
    assert_equal TT::SEMICOLON, tokens[0].type
  end

  # ------------------------------------------------------------------
  # Identifiers and literals
  # ------------------------------------------------------------------

  def test_dollar_sign_identifier
    tokens = tokenize("$foo")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "$foo", tokens[0].value
  end

  def test_hex_number
    tokens = tokenize("0xFF")
    assert_equal TT::NUMBER, tokens[0].type
  end

  def test_float_number
    tokens = tokenize("3.14")
    assert_equal TT::NUMBER, tokens[0].type
  end

  def test_double_quoted_string
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_single_quoted_string
    tokens = tokenize("'world'")
    assert_equal TT::STRING, tokens[0].type
    assert_equal "world", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Comments are skipped
  # ------------------------------------------------------------------

  def test_line_comment_skipped
    tokens = tokenize("var x; // comment")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_equal ["var", "x", ";"], values
  end

  def test_block_comment_skipped
    tokens = tokenize("var /* skip */ x;")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_equal ["var", "x", ";"], values
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::EcmascriptEs3Lexer::ES3_TOKENS_PATH),
      "es3.tokens file should exist at #{CodingAdventures::EcmascriptEs3Lexer::ES3_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Multi-token expressions (ES3 features)
  # ------------------------------------------------------------------

  def test_try_catch_tokens
    tokens = tokenize("try { } catch (e) { }")
    values = tokens.map(&:value)
    assert_includes values, "try"
    assert_includes values, "catch"
  end

  def test_instanceof_expression
    tokens = tokenize("x instanceof Array")
    values = tokens.map(&:value)
    assert_equal ["x", "instanceof", "Array", ""], values
  end
end
