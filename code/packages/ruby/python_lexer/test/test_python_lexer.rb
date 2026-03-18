# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Python Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with python.tokens, correctly tokenizes Python source code.
#
# The key insight being tested: the same lexer engine that handles
# one language can handle any language whose tokens are described
# in a .tokens file. We are not testing the lexer engine itself
# (that is tested in the lexer gem) -- we are testing that the
# Python grammar file correctly describes Python's lexical rules.
# ================================================================

class TestPythonLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  # ------------------------------------------------------------------
  # Helper: tokenize and strip the trailing EOF token for cleaner tests
  # ------------------------------------------------------------------

  def tokenize(source)
    CodingAdventures::PythonLexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Basic arithmetic: x = 1 + 2
  # ------------------------------------------------------------------

  def test_basic_assignment
    tokens = tokenize("x = 1 + 2")
    # Expected: NAME EQUALS NUMBER PLUS NUMBER EOF
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::PLUS, TT::NUMBER, TT::EOF], types
  end

  def test_basic_assignment_values
    tokens = tokenize("x = 1 + 2")
    values = tokens.map(&:value)
    assert_equal ["x", "=", "1", "+", "2", ""], values
  end

  # ------------------------------------------------------------------
  # Python keywords
  # ------------------------------------------------------------------

  def test_keyword_if
    tokens = tokenize("if")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "if", tokens[0].value
  end

  def test_keyword_else
    tokens = tokenize("else")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "else", tokens[0].value
  end

  def test_keyword_def
    tokens = tokenize("def")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "def", tokens[0].value
  end

  def test_keyword_return
    tokens = tokenize("return")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "return", tokens[0].value
  end

  def test_keyword_while
    tokens = tokenize("while")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "while", tokens[0].value
  end

  def test_keyword_for
    tokens = tokenize("for")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "for", tokens[0].value
  end

  def test_keyword_true
    tokens = tokenize("True")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "True", tokens[0].value
  end

  def test_keyword_false
    tokens = tokenize("False")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "False", tokens[0].value
  end

  def test_keyword_none
    tokens = tokenize("None")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "None", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Non-keyword names are NAME tokens, not KEYWORD
  # ------------------------------------------------------------------

  def test_name_not_keyword
    tokens = tokenize("foobar")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "foobar", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Comparison operator: ==
  # ------------------------------------------------------------------

  def test_equals_equals
    tokens = tokenize("x == y")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS_EQUALS, TT::NAME, TT::EOF], types
  end

  # ------------------------------------------------------------------
  # All single-character operators
  # ------------------------------------------------------------------

  def test_plus_operator
    tokens = tokenize("a + b")
    assert_equal TT::PLUS, tokens[1].type
  end

  def test_minus_operator
    tokens = tokenize("a - b")
    assert_equal TT::MINUS, tokens[1].type
  end

  def test_star_operator
    tokens = tokenize("a * b")
    assert_equal TT::STAR, tokens[1].type
  end

  def test_slash_operator
    tokens = tokenize("a / b")
    assert_equal TT::SLASH, tokens[1].type
  end

  # ------------------------------------------------------------------
  # Strings
  # ------------------------------------------------------------------

  def test_string_literal
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_string_with_spaces
    tokens = tokenize('"Hello, World!"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "Hello, World!", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Numbers
  # ------------------------------------------------------------------

  def test_number
    tokens = tokenize("42")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_multi_digit_number
    tokens = tokenize("12345")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "12345", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Delimiters: parentheses, comma, colon
  # ------------------------------------------------------------------

  def test_parentheses
    tokens = tokenize("(x)")
    types = tokens.map(&:type)
    assert_equal [TT::LPAREN, TT::NAME, TT::RPAREN, TT::EOF], types
  end

  def test_comma
    tokens = tokenize("a, b")
    assert_equal TT::COMMA, tokens[1].type
  end

  def test_colon
    tokens = tokenize("if x:")
    assert_equal TT::COLON, tokens[2].type
  end

  # ------------------------------------------------------------------
  # Multi-line Python code
  # ------------------------------------------------------------------

  def test_multiline
    source = "x = 1\ny = 2"
    tokens = tokenize(source)
    types = tokens.map(&:type)
    # x = 1 \n y = 2 EOF
    expected = [
      TT::NAME, TT::EQUALS, TT::NUMBER,
      TT::NEWLINE,
      TT::NAME, TT::EQUALS, TT::NUMBER,
      TT::EOF
    ]
    assert_equal expected, types
  end

  # ------------------------------------------------------------------
  # print("Hello, World!") -- function call syntax
  # ------------------------------------------------------------------

  def test_print_hello_world
    source = 'print("Hello, World!")'
    tokens = tokenize(source)
    types = tokens.map(&:type)
    expected = [TT::NAME, TT::LPAREN, TT::STRING, TT::RPAREN, TT::EOF]
    assert_equal expected, types
    assert_equal "print", tokens[0].value
    assert_equal "Hello, World!", tokens[2].value
  end

  # ------------------------------------------------------------------
  # Complex expression with multiple operators
  # ------------------------------------------------------------------

  def test_complex_expression
    source = "result = a + b * c - d / e"
    tokens = tokenize(source)
    types = tokens.map(&:type)
    expected = [
      TT::NAME, TT::EQUALS,
      TT::NAME, TT::PLUS, TT::NAME, TT::STAR, TT::NAME,
      TT::MINUS, TT::NAME, TT::SLASH, TT::NAME,
      TT::EOF
    ]
    assert_equal expected, types
  end

  # ------------------------------------------------------------------
  # Line and column tracking
  # ------------------------------------------------------------------

  def test_line_tracking
    source = "x = 1\ny = 2"
    tokens = tokenize(source)
    # 'y' should be on line 2
    y_token = tokens.find { |t| t.value == "y" }
    assert_equal 2, y_token.line
  end

  # ------------------------------------------------------------------
  # Keyword in context: def foo(x):
  # ------------------------------------------------------------------

  def test_def_function
    source = "def foo(x):"
    tokens = tokenize(source)
    types = tokens.map(&:type)
    expected = [
      TT::KEYWORD, TT::NAME, TT::LPAREN, TT::NAME, TT::RPAREN,
      TT::COLON, TT::EOF
    ]
    assert_equal expected, types
    assert_equal "def", tokens[0].value
    assert_equal "foo", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Edge: equals vs equals_equals disambiguation
  # ------------------------------------------------------------------

  def test_equals_vs_equals_equals
    source = "x = y == z"
    tokens = tokenize(source)
    types = tokens.map(&:type)
    expected = [TT::NAME, TT::EQUALS, TT::NAME, TT::EQUALS_EQUALS, TT::NAME, TT::EOF]
    assert_equal expected, types
  end

  # ------------------------------------------------------------------
  # All Python keywords from python.tokens
  # ------------------------------------------------------------------

  def test_all_keywords
    keywords = %w[if else elif while for def return class import from as True False None]
    keywords.each do |kw|
      tokens = tokenize(kw)
      assert_equal TT::KEYWORD, tokens[0].type, "Expected '#{kw}' to be a KEYWORD"
      assert_equal kw, tokens[0].value
    end
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::PythonLexer::PYTHON_TOKENS_PATH),
      "python.tokens file should exist at #{CodingAdventures::PythonLexer::PYTHON_TOKENS_PATH}"
  end
end
