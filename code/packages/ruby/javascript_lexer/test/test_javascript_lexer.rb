# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the JavaScript Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with javascript.tokens, correctly tokenizes JavaScript source code.
# ================================================================

class TestJavascriptLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  def tokenize(source)
    CodingAdventures::JavascriptLexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Basic expression: let x = 1 + 2;
  # ------------------------------------------------------------------

  def test_let_assignment
    tokens = tokenize("let x = 1 + 2;")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::NAME, TT::EQUALS, TT::NUMBER, TT::PLUS, TT::NUMBER, TT::SEMICOLON, TT::EOF], types
  end

  def test_let_assignment_values
    tokens = tokenize("let x = 1 + 2;")
    values = tokens.map(&:value)
    assert_equal ["let", "x", "=", "1", "+", "2", ";", ""], values
  end

  # ------------------------------------------------------------------
  # JavaScript keywords
  # ------------------------------------------------------------------

  def test_keyword_let
    tokens = tokenize("let")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "let", tokens[0].value
  end

  def test_keyword_const
    tokens = tokenize("const")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "const", tokens[0].value
  end

  def test_keyword_function
    tokens = tokenize("function")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "function", tokens[0].value
  end

  def test_boolean_and_null_keywords
    tokens = tokenize("true false null undefined")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[true false null undefined], keywords
  end

  def test_name_not_keyword
    tokens = tokenize("foobar")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "foobar", tokens[0].value
  end

  # ------------------------------------------------------------------
  # JavaScript-specific operators
  # ------------------------------------------------------------------

  def test_strict_equality
    tokens = tokenize("x === 1")
    assert_equal "===", tokens[1].value
  end

  def test_strict_inequality
    tokens = tokenize("x !== 1")
    assert_equal "!==", tokens[1].value
  end

  def test_equality
    tokens = tokenize("x == 1")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS_EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_arrow_operator
    tokens = tokenize("x => x")
    assert_equal "=>", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Delimiters
  # ------------------------------------------------------------------

  def test_curly_braces
    tokens = tokenize("{ }")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACE, TT::RBRACE, TT::EOF], types
  end

  def test_square_brackets
    tokens = tokenize("[ ]")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACKET, TT::RBRACKET, TT::EOF], types
  end

  def test_semicolon
    tokens = tokenize(";")
    assert_equal TT::SEMICOLON, tokens[0].type
  end

  # ------------------------------------------------------------------
  # Identifiers with $
  # ------------------------------------------------------------------

  def test_dollar_sign_identifier
    tokens = tokenize("$foo")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "$foo", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Strings
  # ------------------------------------------------------------------

  def test_string_literal
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::JavascriptLexer::JS_TOKENS_PATH),
      "javascript.tokens file should exist at #{CodingAdventures::JavascriptLexer::JS_TOKENS_PATH}"
  end
end
