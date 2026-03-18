# frozen_string_literal: true

require_relative "test_helper"

class TestTokenizer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType
  Tok = CodingAdventures::Lexer::Token
  Tokenizer = CodingAdventures::Lexer::Tokenizer

  # -----------------------------------------------------------------------
  # Basic token types
  # -----------------------------------------------------------------------

  def test_empty_source
    tokens = Tokenizer.new("").tokenize
    assert_equal 1, tokens.length
    assert_equal TT::EOF, tokens[0].type
  end

  def test_single_number
    tokens = Tokenizer.new("42").tokenize
    assert_equal 2, tokens.length
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_single_name
    tokens = Tokenizer.new("hello").tokenize
    assert_equal TT::NAME, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_name_with_underscore
    tokens = Tokenizer.new("my_var").tokenize
    assert_equal TT::NAME, tokens[0].type
    assert_equal "my_var", tokens[0].value
  end

  def test_name_starting_with_underscore
    tokens = Tokenizer.new("_private").tokenize
    assert_equal TT::NAME, tokens[0].type
    assert_equal "_private", tokens[0].value
  end

  def test_string_literal
    tokens = Tokenizer.new('"hello"').tokenize
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_string_with_escapes
    tokens = Tokenizer.new('"line1\\nline2"').tokenize
    assert_equal "line1\nline2", tokens[0].value
  end

  def test_string_with_tab_escape
    tokens = Tokenizer.new('"col1\\tcol2"').tokenize
    assert_equal "col1\tcol2", tokens[0].value
  end

  def test_string_with_escaped_quote
    tokens = Tokenizer.new('"say \\"hi\\""').tokenize
    assert_equal 'say "hi"', tokens[0].value
  end

  def test_string_with_escaped_backslash
    tokens = Tokenizer.new('"path\\\\file"').tokenize
    assert_equal 'path\\file', tokens[0].value
  end

  # -----------------------------------------------------------------------
  # Operators
  # -----------------------------------------------------------------------

  def test_operators
    tokens = Tokenizer.new("+ - * /").tokenize
    types = tokens.map(&:type)
    assert_equal [TT::PLUS, TT::MINUS, TT::STAR, TT::SLASH, TT::EOF], types
  end

  def test_equals
    tokens = Tokenizer.new("=").tokenize
    assert_equal TT::EQUALS, tokens[0].type
  end

  def test_equals_equals
    tokens = Tokenizer.new("==").tokenize
    assert_equal TT::EQUALS_EQUALS, tokens[0].type
    assert_equal "==", tokens[0].value
  end

  def test_delimiters
    tokens = Tokenizer.new("( ) , :").tokenize
    types = tokens.map(&:type)
    assert_equal [TT::LPAREN, TT::RPAREN, TT::COMMA, TT::COLON, TT::EOF], types
  end

  # -----------------------------------------------------------------------
  # Keywords
  # -----------------------------------------------------------------------

  def test_keyword_recognition
    tokens = Tokenizer.new("if", keywords: ["if"]).tokenize
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "if", tokens[0].value
  end

  def test_non_keyword_name
    tokens = Tokenizer.new("iffy", keywords: ["if"]).tokenize
    assert_equal TT::NAME, tokens[0].type
  end

  def test_multiple_keywords
    tokens = Tokenizer.new("if else while", keywords: %w[if else while]).tokenize
    types = tokens[0..2].map(&:type)
    assert_equal [TT::KEYWORD, TT::KEYWORD, TT::KEYWORD], types
  end

  # -----------------------------------------------------------------------
  # Whitespace and newlines
  # -----------------------------------------------------------------------

  def test_skip_whitespace
    tokens = Tokenizer.new("x   +   y").tokenize
    assert_equal 4, tokens.length # NAME PLUS NAME EOF
  end

  def test_newline_token
    tokens = Tokenizer.new("x\ny").tokenize
    assert_equal TT::NAME, tokens[0].type
    assert_equal TT::NEWLINE, tokens[1].type
    assert_equal TT::NAME, tokens[2].type
  end

  def test_skip_carriage_return
    tokens = Tokenizer.new("x\r\ny").tokenize
    assert_equal TT::NAME, tokens[0].type
    assert_equal TT::NEWLINE, tokens[1].type
    assert_equal TT::NAME, tokens[2].type
  end

  # -----------------------------------------------------------------------
  # Line and column tracking
  # -----------------------------------------------------------------------

  def test_position_tracking
    tokens = Tokenizer.new("x = 1").tokenize
    assert_equal 1, tokens[0].line
    assert_equal 1, tokens[0].column
    assert_equal 1, tokens[1].line
    assert_equal 3, tokens[1].column
    assert_equal 1, tokens[2].line
    assert_equal 5, tokens[2].column
  end

  def test_multiline_position
    tokens = Tokenizer.new("x\ny").tokenize
    assert_equal 1, tokens[0].line # x
    assert_equal 2, tokens[2].line # y
  end

  # -----------------------------------------------------------------------
  # Complete expressions
  # -----------------------------------------------------------------------

  def test_assignment
    tokens = Tokenizer.new("x = 1 + 2").tokenize
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::PLUS, TT::NUMBER, TT::EOF], types
  end

  def test_comparison
    tokens = Tokenizer.new("x == 5").tokenize
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS_EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_complex_expression
    tokens = Tokenizer.new("(x + 1) * y").tokenize
    types = tokens.map(&:type)
    expected = [TT::LPAREN, TT::NAME, TT::PLUS, TT::NUMBER, TT::RPAREN,
                TT::STAR, TT::NAME, TT::EOF]
    assert_equal expected, types
  end

  # -----------------------------------------------------------------------
  # Error handling
  # -----------------------------------------------------------------------

  def test_unexpected_character
    error = assert_raises(CodingAdventures::Lexer::LexerError) do
      Tokenizer.new("@").tokenize
    end
    assert_includes error.message, "Unexpected character"
    assert_equal 1, error.line
    assert_equal 1, error.column
  end

  def test_unterminated_string
    error = assert_raises(CodingAdventures::Lexer::LexerError) do
      Tokenizer.new('"hello').tokenize
    end
    assert_includes error.message, "Unterminated"
  end

  def test_unterminated_string_with_backslash
    error = assert_raises(CodingAdventures::Lexer::LexerError) do
      Tokenizer.new('"hello\\').tokenize
    end
    assert_includes error.message, "Unterminated"
  end

  # -----------------------------------------------------------------------
  # Token to_s
  # -----------------------------------------------------------------------

  def test_token_to_s
    token = Tok.new(type: TT::NAME, value: "x", line: 1, column: 1)
    assert_includes token.to_s, "NAME"
    assert_includes token.to_s, "x"
  end
end
