# frozen_string_literal: true

require_relative "test_helper"

class TestGrammarLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType
  GL = CodingAdventures::Lexer::GrammarLexer
  GT = CodingAdventures::GrammarTools

  def make_grammar(source)
    GT.parse_token_grammar(source)
  end

  def simple_grammar
    make_grammar(<<~TOKENS)
      NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
      NUMBER = /[0-9]+/
      STRING = /"([^"\\\\]|\\\\.)*"/
      EQUALS_EQUALS = "=="
      EQUALS = "="
      PLUS   = "+"
      MINUS  = "-"
      STAR   = "*"
      SLASH  = "/"
      LPAREN = "("
      RPAREN = ")"
      COMMA  = ","
      COLON  = ":"

      keywords:
        if
        else
    TOKENS
  end

  # -----------------------------------------------------------------------
  # Basic tokenization
  # -----------------------------------------------------------------------

  def test_empty_source
    tokens = GL.new("", simple_grammar).tokenize
    assert_equal 1, tokens.length
    assert_equal TT::EOF, tokens[0].type
  end

  def test_single_number
    tokens = GL.new("42", simple_grammar).tokenize
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_single_name
    tokens = GL.new("hello", simple_grammar).tokenize
    assert_equal TT::NAME, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_operators
    tokens = GL.new("+ - * /", simple_grammar).tokenize
    types = tokens.map(&:type)
    assert_equal [TT::PLUS, TT::MINUS, TT::STAR, TT::SLASH, TT::EOF], types
  end

  def test_equals_vs_equals_equals
    tokens = GL.new("= ==", simple_grammar).tokenize
    assert_equal TT::EQUALS, tokens[0].type
    assert_equal TT::EQUALS_EQUALS, tokens[1].type
  end

  def test_string_literal
    tokens = GL.new('"hello"', simple_grammar).tokenize
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_string_with_escapes
    tokens = GL.new('"line1\\nline2"', simple_grammar).tokenize
    assert_equal "line1\nline2", tokens[0].value
  end

  # -----------------------------------------------------------------------
  # Keywords
  # -----------------------------------------------------------------------

  def test_keyword_detection
    tokens = GL.new("if", simple_grammar).tokenize
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "if", tokens[0].value
  end

  def test_non_keyword
    tokens = GL.new("iffy", simple_grammar).tokenize
    assert_equal TT::NAME, tokens[0].type
  end

  # -----------------------------------------------------------------------
  # Newlines
  # -----------------------------------------------------------------------

  def test_newline_handling
    tokens = GL.new("x\ny", simple_grammar).tokenize
    assert_equal TT::NAME, tokens[0].type
    assert_equal TT::NEWLINE, tokens[1].type
    assert_equal TT::NAME, tokens[2].type
  end

  # -----------------------------------------------------------------------
  # Position tracking
  # -----------------------------------------------------------------------

  def test_position_tracking
    tokens = GL.new("x = 1", simple_grammar).tokenize
    assert_equal 1, tokens[0].line
    assert_equal 1, tokens[0].column
    assert_equal 1, tokens[1].line
    assert_equal 3, tokens[1].column
  end

  # -----------------------------------------------------------------------
  # Complete expressions
  # -----------------------------------------------------------------------

  def test_assignment
    tokens = GL.new("x = 1 + 2", simple_grammar).tokenize
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::PLUS, TT::NUMBER, TT::EOF], types
  end

  # -----------------------------------------------------------------------
  # Error handling
  # -----------------------------------------------------------------------

  def test_unexpected_character
    error = assert_raises(CodingAdventures::Lexer::LexerError) do
      GL.new("@", simple_grammar).tokenize
    end
    assert_includes error.message, "Unexpected"
  end

  # -----------------------------------------------------------------------
  # Fallback token type
  # -----------------------------------------------------------------------

  def test_unknown_token_name_falls_back_to_name
    grammar = make_grammar('CUSTOM = /[a-z]+/')
    tokens = GL.new("hello", grammar).tokenize
    # CUSTOM is not in TokenType::ALL so falls back to NAME
    assert_equal TT::NAME, tokens[0].type
  end

  # -----------------------------------------------------------------------
  # Delimiters
  # -----------------------------------------------------------------------

  def test_delimiters
    tokens = GL.new("( ) , :", simple_grammar).tokenize
    types = tokens.map(&:type)
    assert_equal [TT::LPAREN, TT::RPAREN, TT::COMMA, TT::COLON, TT::EOF], types
  end
end
