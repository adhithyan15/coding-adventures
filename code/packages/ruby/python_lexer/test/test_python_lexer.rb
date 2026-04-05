# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Python Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with versioned python.tokens files, correctly tokenizes Python
# source code for multiple Python versions.
#
# The key insight being tested: the same lexer engine that handles
# one language can handle any language whose tokens are described
# in a .tokens file. We are not testing the lexer engine itself
# (that is tested in the lexer gem) -- we are testing that the
# Python grammar files correctly describe Python's lexical rules.
# ================================================================

class TestPythonLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  # Clear the grammar cache before each test so tests are independent.
  def setup
    CodingAdventures::PythonLexer.clear_cache!
  end

  # ------------------------------------------------------------------
  # Helper: tokenize and strip the trailing EOF token for cleaner tests
  # ------------------------------------------------------------------

  def tokenize(source, version: "3.12")
    CodingAdventures::PythonLexer.tokenize(source, version: version)
  end

  def token_types(source, version: "3.12")
    tokenize(source, version: version).map(&:type)
  end

  def token_values(source, version: "3.12")
    tokenize(source, version: version).map(&:value)
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
  # Version parameter and grammar path resolution
  # ------------------------------------------------------------------

  def test_default_version_is_3_12
    assert_equal "3.12", CodingAdventures::PythonLexer::DEFAULT_VERSION
  end

  def test_supported_versions
    expected = %w[2.7 3.0 3.6 3.8 3.10 3.12]
    assert_equal expected, CodingAdventures::PythonLexer::SUPPORTED_VERSIONS
  end

  def test_grammar_path_for_version
    path = CodingAdventures::PythonLexer.grammar_path("3.12")
    assert path.end_with?("python/python3.12.tokens"),
      "Expected path to end with python/python3.12.tokens, got: #{path}"
  end

  def test_grammar_path_for_2_7
    path = CodingAdventures::PythonLexer.grammar_path("2.7")
    assert path.end_with?("python/python2.7.tokens"),
      "Expected path to end with python/python2.7.tokens, got: #{path}"
  end

  def test_grammar_files_exist_for_all_versions
    CodingAdventures::PythonLexer::SUPPORTED_VERSIONS.each do |v|
      path = CodingAdventures::PythonLexer.grammar_path(v)
      assert File.exist?(path),
        "Grammar file missing for Python #{v}: #{path}"
    end
  end

  def test_unsupported_version_raises
    assert_raises(ArgumentError) do
      CodingAdventures::PythonLexer.tokenize("x = 1", version: "1.0")
    end
  end

  def test_unsupported_version_error_message
    error = assert_raises(ArgumentError) do
      CodingAdventures::PythonLexer.tokenize("x = 1", version: "99.9")
    end
    assert_includes error.message, "Unsupported Python version"
    assert_includes error.message, "99.9"
  end

  def test_tokenize_with_default_version
    # Calling without version: should use 3.12
    tokens = CodingAdventures::PythonLexer.tokenize("x = 1")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_tokenize_with_explicit_3_12
    tokens = tokenize("x = 1", version: "3.12")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_tokenize_with_3_8
    tokens = tokenize("x = 1", version: "3.8")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_tokenize_with_2_7
    tokens = tokenize("x = 1", version: "2.7")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_tokenize_with_3_0
    tokens = tokenize("x = 1", version: "3.0")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_tokenize_with_3_6
    tokens = tokenize("x = 1", version: "3.6")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_tokenize_with_3_10
    tokens = tokenize("x = 1", version: "3.10")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_grammar_caching
    # First call loads the grammar, second call should use the cache.
    CodingAdventures::PythonLexer.tokenize("x = 1", version: "3.12")
    CodingAdventures::PythonLexer.tokenize("y = 2", version: "3.12")
    # No assertion needed beyond not crashing -- caching is an
    # implementation detail. We verify it works by the fact that
    # the second call produces correct results.
    tokens = CodingAdventures::PythonLexer.tokenize("y = 2", version: "3.12")
    assert_equal "y", tokens[0].value
  end

  def test_clear_cache
    CodingAdventures::PythonLexer.tokenize("x = 1", version: "3.12")
    CodingAdventures::PythonLexer.clear_cache!
    # After clearing, the next call should re-load from disk.
    tokens = CodingAdventures::PythonLexer.tokenize("x = 1", version: "3.12")
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::EOF],
      tokens.map(&:type)
  end

  def test_different_versions_cached_independently
    CodingAdventures::PythonLexer.tokenize("x = 1", version: "3.8")
    CodingAdventures::PythonLexer.tokenize("x = 1", version: "3.12")
    # Both should work without interference.
    tokens_38 = CodingAdventures::PythonLexer.tokenize("x = 1", version: "3.8")
    tokens_312 = CodingAdventures::PythonLexer.tokenize("x = 1", version: "3.12")
    assert_equal "x", tokens_38[0].value
    assert_equal "x", tokens_312[0].value
  end
end
