# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Java Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with java/java<version>.tokens, correctly tokenizes Java source code.
#
# Version-aware tests verify that the `version:` keyword argument
# selects the correct versioned Java grammar from
# code/grammars/java/.
# ================================================================

class TestJavaLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  def tokenize(source, version: nil)
    CodingAdventures::JavaLexer.tokenize(source, version: version)
  end

  def token_types(source, version: nil)
    tokenize(source, version: version).map(&:type)
  end

  def token_values(source, version: nil)
    tokenize(source, version: version).map(&:value)
  end

  # ------------------------------------------------------------------
  # Basic expression: int x = 1 + 2;
  # ------------------------------------------------------------------

  def test_int_assignment
    tokens = tokenize("int x = 1 + 2;")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::NAME, TT::EQUALS, TT::NUMBER, TT::PLUS, TT::NUMBER, TT::SEMICOLON, TT::EOF], types
  end

  def test_int_assignment_values
    tokens = tokenize("int x = 1 + 2;")
    values = tokens.map(&:value)
    assert_equal ["int", "x", "=", "1", "+", "2", ";", ""], values
  end

  # ------------------------------------------------------------------
  # Java keywords
  # ------------------------------------------------------------------

  def test_keyword_class
    tokens = tokenize("class")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "class", tokens[0].value
  end

  def test_keyword_public
    tokens = tokenize("public")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "public", tokens[0].value
  end

  def test_keyword_static
    tokens = tokenize("static")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "static", tokens[0].value
  end

  def test_boolean_keywords
    tokens = tokenize("true false null")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[true false null], keywords
  end

  def test_name_not_keyword
    tokens = tokenize("foobar")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "foobar", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Operators
  # ------------------------------------------------------------------

  def test_equality
    tokens = tokenize("x == 1")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS_EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_not_equals
    tokens = tokenize("x != 1")
    assert_equal "!=", tokens[1].value
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
  # Strings
  # ------------------------------------------------------------------

  def test_string_literal
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Version-aware: nil uses default (Java 21) grammar
  # ------------------------------------------------------------------

  def test_no_version_uses_default_grammar
    path = CodingAdventures::JavaLexer.resolve_tokens_path(nil)
    assert_match(%r{java/java21\.tokens$}, path)
    assert File.exist?(path), "java21.tokens file should exist at #{path}"
  end

  def test_empty_string_version_uses_default_grammar
    path = CodingAdventures::JavaLexer.resolve_tokens_path("")
    assert_match(%r{java/java21\.tokens$}, path)
  end

  # ------------------------------------------------------------------
  # Version-aware: valid version strings resolve to versioned paths
  # ------------------------------------------------------------------

  def test_resolve_tokens_path_1_0
    path = CodingAdventures::JavaLexer.resolve_tokens_path("1.0")
    assert_match(%r{java/java1\.0\.tokens$}, path)
    assert File.exist?(path), "java1.0.tokens should exist at #{path}"
  end

  def test_resolve_tokens_path_8
    path = CodingAdventures::JavaLexer.resolve_tokens_path("8")
    assert_match(%r{java/java8\.tokens$}, path)
    assert File.exist?(path), "java8.tokens should exist at #{path}"
  end

  def test_resolve_tokens_path_17
    path = CodingAdventures::JavaLexer.resolve_tokens_path("17")
    assert_match(%r{java/java17\.tokens$}, path)
    assert File.exist?(path), "java17.tokens should exist at #{path}"
  end

  def test_resolve_tokens_path_21
    path = CodingAdventures::JavaLexer.resolve_tokens_path("21")
    assert_match(%r{java/java21\.tokens$}, path)
    assert File.exist?(path), "java21.tokens should exist at #{path}"
  end

  # ------------------------------------------------------------------
  # Version-aware: all valid versions have grammar files on disk
  # ------------------------------------------------------------------

  def test_all_valid_versions_have_tokens_files
    CodingAdventures::JavaLexer::VALID_VERSIONS.each do |version|
      path = CodingAdventures::JavaLexer.resolve_tokens_path(version)
      assert File.exist?(path),
        "Grammar file for version #{version.inspect} should exist at #{path}"
    end
  end

  # ------------------------------------------------------------------
  # Version-aware: tokenize with an explicit version
  # ------------------------------------------------------------------

  def test_tokenize_with_java_8_version
    tokens = tokenize("int x = 1;", version: "8")
    values = tokens.map(&:value)
    assert_includes values, "int"
    assert_includes values, "x"
  end

  def test_tokenize_with_java_17_version
    tokens = tokenize("int x = 1;", version: "17")
    values = tokens.map(&:value)
    assert_includes values, "int"
  end

  def test_tokenize_with_java_1_0_version
    tokens = tokenize("int x = 1;", version: "1.0")
    values = tokens.map(&:value)
    assert_includes values, "int"
  end

  # ------------------------------------------------------------------
  # Version-aware: unknown version raises ArgumentError
  # ------------------------------------------------------------------

  def test_unknown_version_raises_argument_error
    err = assert_raises(ArgumentError) do
      tokenize("int x = 1;", version: "99")
    end
    assert_match(/99/, err.message)
    assert_match(/Valid versions/, err.message)
  end

  def test_unknown_version_error_lists_valid_versions
    err = assert_raises(ArgumentError) do
      CodingAdventures::JavaLexer.resolve_tokens_path("bogus")
    end
    CodingAdventures::JavaLexer::VALID_VERSIONS.each do |v|
      assert_match(/#{Regexp.escape(v)}/, err.message)
    end
  end

  # ------------------------------------------------------------------
  # Backward compatibility: tokenize with no version arg still works
  # ------------------------------------------------------------------

  def test_backward_compatible_no_version
    tokens = CodingAdventures::JavaLexer.tokenize("int x = 1;")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "int", tokens[0].value
  end

  # ------------------------------------------------------------------
  # create_lexer factory method
  # ------------------------------------------------------------------

  def test_create_lexer_returns_hash
    result = CodingAdventures::JavaLexer.create_lexer("int x = 1;")
    assert_instance_of Hash, result
  end

  def test_create_lexer_stores_source
    result = CodingAdventures::JavaLexer.create_lexer("int x = 1;")
    assert_equal "int x = 1;", result[:source]
  end

  def test_create_lexer_stores_nil_version
    result = CodingAdventures::JavaLexer.create_lexer("int x = 1;")
    assert_nil result[:version]
  end

  def test_create_lexer_stores_language
    result = CodingAdventures::JavaLexer.create_lexer("int x = 1;")
    assert_equal :java, result[:language]
  end

  def test_create_lexer_with_version
    result = CodingAdventures::JavaLexer.create_lexer("int x = 1;", version: "8")
    assert_equal "8", result[:version]
  end

  def test_create_lexer_raises_for_unknown_version
    assert_raises(ArgumentError) do
      CodingAdventures::JavaLexer.create_lexer("int x = 1;", version: "bogus")
    end
  end
end
