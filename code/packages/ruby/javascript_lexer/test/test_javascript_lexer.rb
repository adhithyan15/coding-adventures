# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the JavaScript Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with javascript.tokens, correctly tokenizes JavaScript source code.
#
# Version-aware tests verify that the `version:` keyword argument
# selects the correct versioned ECMAScript grammar from
# code/grammars/ecmascript/.
# ================================================================

class TestJavascriptLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  def tokenize(source, version: nil)
    CodingAdventures::JavascriptLexer.tokenize(source, version: version)
  end

  def token_types(source, version: nil)
    tokenize(source, version: version).map(&:type)
  end

  def token_values(source, version: nil)
    tokenize(source, version: version).map(&:value)
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
  # Grammar path resolution (generic)
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::JavascriptLexer::JS_TOKENS_PATH),
      "javascript.tokens file should exist at #{CodingAdventures::JavascriptLexer::JS_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Version-aware: nil and empty string both use generic grammar
  # ------------------------------------------------------------------

  def test_no_version_uses_generic_grammar
    path = CodingAdventures::JavascriptLexer.resolve_tokens_path(nil)
    assert_match(/javascript\.tokens$/, path)
    assert File.exist?(path), "Generic javascript.tokens should exist"
  end

  def test_empty_string_version_uses_generic_grammar
    path = CodingAdventures::JavascriptLexer.resolve_tokens_path("")
    assert_match(/javascript\.tokens$/, path)
  end

  # ------------------------------------------------------------------
  # Version-aware: valid version strings resolve to versioned paths
  # ------------------------------------------------------------------

  def test_resolve_tokens_path_es1
    path = CodingAdventures::JavascriptLexer.resolve_tokens_path("es1")
    assert_match(%r{ecmascript/es1\.tokens$}, path)
    assert File.exist?(path), "es1.tokens should exist at #{path}"
  end

  def test_resolve_tokens_path_es5
    path = CodingAdventures::JavascriptLexer.resolve_tokens_path("es5")
    assert_match(%r{ecmascript/es5\.tokens$}, path)
    assert File.exist?(path), "es5.tokens should exist at #{path}"
  end

  def test_resolve_tokens_path_es2020
    path = CodingAdventures::JavascriptLexer.resolve_tokens_path("es2020")
    assert_match(%r{ecmascript/es2020\.tokens$}, path)
    assert File.exist?(path), "es2020.tokens should exist at #{path}"
  end

  def test_resolve_tokens_path_es2025
    path = CodingAdventures::JavascriptLexer.resolve_tokens_path("es2025")
    assert_match(%r{ecmascript/es2025\.tokens$}, path)
    assert File.exist?(path), "es2025.tokens should exist at #{path}"
  end

  # ------------------------------------------------------------------
  # Version-aware: all valid versions have grammar files on disk
  # ------------------------------------------------------------------

  def test_all_valid_versions_have_tokens_files
    CodingAdventures::JavascriptLexer::VALID_VERSIONS.each do |version|
      path = CodingAdventures::JavascriptLexer.resolve_tokens_path(version)
      assert File.exist?(path),
        "Grammar file for version #{version.inspect} should exist at #{path}"
    end
  end

  # ------------------------------------------------------------------
  # Version-aware: tokenize with an explicit version
  # ------------------------------------------------------------------

  def test_tokenize_with_es2020_version
    tokens = tokenize("let x = 1;", version: "es2020")
    values = tokens.map(&:value)
    assert_includes values, "let"
    assert_includes values, "x"
  end

  def test_tokenize_with_es5_version
    tokens = tokenize("var x = 1;", version: "es5")
    values = tokens.map(&:value)
    assert_includes values, "var"
  end

  def test_tokenize_with_es1_version
    tokens = tokenize("var x = 1;", version: "es1")
    values = tokens.map(&:value)
    assert_includes values, "var"
  end

  # ------------------------------------------------------------------
  # Version-aware: unknown version raises ArgumentError
  # ------------------------------------------------------------------

  def test_unknown_version_raises_argument_error
    err = assert_raises(ArgumentError) do
      tokenize("let x = 1;", version: "es9999")
    end
    assert_match(/es9999/, err.message)
    assert_match(/Valid versions/, err.message)
  end

  def test_unknown_version_error_lists_valid_versions
    err = assert_raises(ArgumentError) do
      CodingAdventures::JavascriptLexer.resolve_tokens_path("bogus")
    end
    CodingAdventures::JavascriptLexer::VALID_VERSIONS.each do |v|
      assert_match(/#{Regexp.escape(v)}/, err.message)
    end
  end

  # ------------------------------------------------------------------
  # Backward compatibility: tokenize with no version arg still works
  # ------------------------------------------------------------------

  def test_backward_compatible_no_version
    tokens = CodingAdventures::JavascriptLexer.tokenize("let x = 1;")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "let", tokens[0].value
  end
end
