# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the ECMAScript 5 (ES5) Lexer
# ================================================================
#
# ES5 (2009) is lexically similar to ES3 with one key change:
# `debugger` is promoted from a future-reserved word to a keyword.
# These tests verify ES5-specific features and confirm backward
# compatibility with ES3 features.
# ================================================================

class TestEcmascriptEs5Lexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  def tokenize(source)
    CodingAdventures::EcmascriptEs5Lexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Basic expression: var x = 1 + 2;
  # ------------------------------------------------------------------

  def test_var_assignment
    tokens = tokenize("var x = 1 + 2;")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::NAME, TT::EQUALS, TT::NUMBER,
      TT::PLUS, TT::NUMBER, TT::SEMICOLON, TT::EOF], types
  end

  # ------------------------------------------------------------------
  # ES5-specific: debugger keyword
  # ------------------------------------------------------------------

  def test_keyword_debugger
    tokens = tokenize("debugger")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "debugger", tokens[0].value
  end

  def test_debugger_statement
    tokens = tokenize("debugger;")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::SEMICOLON, TT::EOF], types
    assert_equal "debugger", tokens[0].value
  end

  # ------------------------------------------------------------------
  # ES3 features retained in ES5
  # ------------------------------------------------------------------

  def test_strict_equals
    tokens = tokenize("x === 1")
    assert_equal "===", tokens[1].value
  end

  def test_strict_not_equals
    tokens = tokenize("x !== 1")
    assert_equal "!==", tokens[1].value
  end

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

  def test_regex_literal
    tokens = tokenize("/abc/g")
    assert_equal "REGEX", tokens[0].type
  end

  # ------------------------------------------------------------------
  # ES1 features retained
  # ------------------------------------------------------------------

  def test_keyword_var
    tokens = tokenize("var")
    assert_equal TT::KEYWORD, tokens[0].type
  end

  def test_keyword_function
    tokens = tokenize("function")
    assert_equal TT::KEYWORD, tokens[0].type
  end

  def test_boolean_and_null_keywords
    tokens = tokenize("true false null")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[true false null], keywords
  end

  def test_equality_operators
    tokens = tokenize("x == 1")
    assert_equal "==", tokens[1].value
  end

  def test_not_equals
    tokens = tokenize("x != 1")
    assert_equal "!=", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Operators
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
    tokens = tokenize("a && b")
    assert_equal "&&", tokens[1].value
  end

  def test_increment_decrement
    tokens = tokenize("x++")
    assert_equal "++", tokens[1].value
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
  # Literals
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

  def test_string_literal
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Comments
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
    assert File.exist?(CodingAdventures::EcmascriptEs5Lexer::ES5_TOKENS_PATH),
      "es5.tokens file should exist at #{CodingAdventures::EcmascriptEs5Lexer::ES5_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # ES5 "use strict" is just a string (lexer doesn't treat it specially)
  # ------------------------------------------------------------------

  def test_use_strict_is_string
    tokens = tokenize('"use strict";')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "use strict", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Getter/setter syntax uses NAME tokens (not keywords)
  # ------------------------------------------------------------------

  def test_get_is_name_not_keyword
    tokens = tokenize("get")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "get", tokens[0].value
  end

  def test_set_is_name_not_keyword
    tokens = tokenize("set")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "set", tokens[0].value
  end
end
