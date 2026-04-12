# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the C# Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with csharp/csharp<version>.tokens, correctly tokenizes C# source
# code.
#
# C# is a statically typed, object-oriented language developed by
# Microsoft as part of the .NET platform.  It shares many keywords
# with Java (`class`, `public`, `static`) but also has C#-specific
# operators and keywords that differ:
#   - `??`  -- null-coalescing operator (return left if not null)
#   - `?.`  -- null-conditional (member access without NPE)
#   - `=>`  -- lambda arrow and expression-bodied member syntax
#   - `var` -- implicitly typed local variable (C# 3.0+)
#   - `async` / `await` -- async/await concurrency model (C# 5.0+)
#   - `namespace` / `using` -- C# module system
#
# Version-aware tests verify that the `version:` keyword argument
# selects the correct versioned C# grammar from
# code/grammars/csharp/.
# ================================================================

class TestCSharpLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  def tokenize(source, version: nil)
    CodingAdventures::CSharpLexer.tokenize(source, version: version)
  end

  def token_types(source, version: nil)
    tokenize(source, version: version).map(&:type)
  end

  def token_values(source, version: nil)
    tokenize(source, version: version).map(&:value)
  end

  # ------------------------------------------------------------------
  # Basic class declaration
  # ------------------------------------------------------------------
  #
  # The simplest meaningful unit of C# is a class declaration.
  # Here we check that `class Foo { }` produces the expected token
  # sequence: KEYWORD("class") NAME("Foo") LBRACE RBRACE EOF.

  def test_basic_class_declaration
    tokens = tokenize("class Foo { }")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::NAME, TT::LBRACE, TT::RBRACE, TT::EOF], types
  end

  def test_class_declaration_values
    tokens = tokenize("class Foo { }")
    values = tokens.map(&:value)
    assert_equal ["class", "Foo", "{", "}", ""], values
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
  # C# keywords
  # ------------------------------------------------------------------
  #
  # C# has an extensive keyword set.  We verify that the grammar file
  # correctly classifies each of these as KEYWORD, not NAME.

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

  def test_keyword_namespace
    # `namespace` is the C# equivalent of Java's `package`
    tokens = tokenize("namespace")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "namespace", tokens[0].value
  end

  def test_keyword_using
    # `using` is the C# equivalent of Java's `import`
    tokens = tokenize("using")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "using", tokens[0].value
  end

  def test_keyword_var
    # `var` is a contextual keyword in C# 3.0+, so the lexer still emits NAME.
    tokens = tokenize("var", version: "3.0")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "var", tokens[0].value
  end

  def test_boolean_keywords
    tokens = tokenize("true false null")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[true false null], keywords
  end

  def test_name_not_keyword
    # A plain identifier should be NAME, not KEYWORD
    tokens = tokenize("foobar")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "foobar", tokens[0].value
  end

  # ------------------------------------------------------------------
  # C#-specific operators
  # ------------------------------------------------------------------
  #
  # These operators distinguish C# from Java.  They are especially
  # important for correctness because they are multi-character
  # tokens that overlap with simpler single-character tokens.
  #
  #   ??   -- null-coalescing: `a ?? b` returns b if a is null
  #   ?.   -- null-conditional: `obj?.Prop` is null if obj is null
  #   =>   -- lambda arrow: `x => x + 1` or expression body
  #           Note: the tokenizer may emit this as ARROW or FAT_ARROW
  #           depending on the grammar version; we test the value.

  def test_null_coalescing_operator
    # `??` must not be tokenized as two separate `?` tokens
    tokens = tokenize("a ?? b")
    op = tokens.find { |t| t.value == "??" }
    refute_nil op, "Expected a ?? token in the stream"
  end

  def test_lambda_arrow_operator
    # `=>` must be recognized as a single operator token
    tokens = tokenize("x => x", version: "3.0")
    op = tokens.find { |t| t.value == "=>" }
    refute_nil op, "Expected a => token in the stream"
  end

  # ------------------------------------------------------------------
  # Equality operators
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
  # Version-aware: nil uses default (C# 12.0) grammar
  # ------------------------------------------------------------------

  def test_no_version_uses_default_grammar
    path = CodingAdventures::CSharpLexer.resolve_tokens_path(nil)
    assert_match(%r{csharp/csharp12\.0\.tokens$}, path)
    assert File.exist?(path), "csharp12.0.tokens file should exist at #{path}"
  end

  def test_empty_string_version_uses_default_grammar
    path = CodingAdventures::CSharpLexer.resolve_tokens_path("")
    assert_match(%r{csharp/csharp12\.0\.tokens$}, path)
  end

  # ------------------------------------------------------------------
  # Version-aware: valid version strings resolve to versioned paths
  # ------------------------------------------------------------------

  def test_resolve_tokens_path_1_0
    path = CodingAdventures::CSharpLexer.resolve_tokens_path("1.0")
    assert_match(%r{csharp/csharp1\.0\.tokens$}, path)
    assert File.exist?(path), "csharp1.0.tokens should exist at #{path}"
  end

  def test_resolve_tokens_path_5_0
    path = CodingAdventures::CSharpLexer.resolve_tokens_path("5.0")
    assert_match(%r{csharp/csharp5\.0\.tokens$}, path)
    assert File.exist?(path), "csharp5.0.tokens should exist at #{path}"
  end

  def test_resolve_tokens_path_8_0
    path = CodingAdventures::CSharpLexer.resolve_tokens_path("8.0")
    assert_match(%r{csharp/csharp8\.0\.tokens$}, path)
    assert File.exist?(path), "csharp8.0.tokens should exist at #{path}"
  end

  def test_resolve_tokens_path_12_0
    path = CodingAdventures::CSharpLexer.resolve_tokens_path("12.0")
    assert_match(%r{csharp/csharp12\.0\.tokens$}, path)
    assert File.exist?(path), "csharp12.0.tokens should exist at #{path}"
  end

  # ------------------------------------------------------------------
  # Version-aware: all 12 valid versions have grammar files on disk
  # ------------------------------------------------------------------

  def test_all_valid_versions_have_tokens_files
    CodingAdventures::CSharpLexer::VALID_VERSIONS.each do |version|
      path = CodingAdventures::CSharpLexer.resolve_tokens_path(version)
      assert File.exist?(path),
        "Grammar file for version #{version.inspect} should exist at #{path}"
    end
  end

  def test_valid_versions_count
    # Exactly 12 versions: 1.0 through 12.0
    assert_equal 12, CodingAdventures::CSharpLexer::VALID_VERSIONS.length
  end

  # ------------------------------------------------------------------
  # Version-aware: tokenize with explicit versions
  # ------------------------------------------------------------------

  def test_tokenize_with_csharp_1_0_version
    tokens = tokenize("int x = 1;", version: "1.0")
    values = tokens.map(&:value)
    assert_includes values, "int"
    assert_includes values, "x"
  end

  def test_tokenize_with_csharp_8_0_version
    tokens = tokenize("int x = 1;", version: "8.0")
    values = tokens.map(&:value)
    assert_includes values, "int"
  end

  def test_tokenize_with_csharp_12_0_version
    tokens = tokenize("class Foo { }", version: "12.0")
    values = tokens.map(&:value)
    assert_includes values, "class"
    assert_includes values, "Foo"
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
      CodingAdventures::CSharpLexer.resolve_tokens_path("bogus")
    end
    CodingAdventures::CSharpLexer::VALID_VERSIONS.each do |v|
      assert_match(/#{Regexp.escape(v)}/, err.message)
    end
  end

  def test_unknown_version_with_java_style_integer_raises
    # Java uses bare integers like "8" but C# always uses dotted "8.0"
    err = assert_raises(ArgumentError) do
      tokenize("int x = 1;", version: "8")
    end
    assert_match(/8/, err.message)
  end

  # ------------------------------------------------------------------
  # Backward compatibility: tokenize with no version arg still works
  # ------------------------------------------------------------------

  def test_backward_compatible_no_version
    tokens = CodingAdventures::CSharpLexer.tokenize("class Foo { }")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "class", tokens[0].value
  end

  # ------------------------------------------------------------------
  # tokenize_csharp alias
  # ------------------------------------------------------------------

  def test_tokenize_csharp_alias_works
    tokens = CodingAdventures::CSharpLexer.tokenize_csharp("class Foo { }")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "class", tokens[0].value
  end

  def test_tokenize_csharp_alias_accepts_version
    tokens = CodingAdventures::CSharpLexer.tokenize_csharp("int x = 1;", version: "5.0")
    values = tokens.map(&:value)
    assert_includes values, "int"
  end

  # ------------------------------------------------------------------
  # create_csharp_lexer factory method
  # ------------------------------------------------------------------

  def test_create_csharp_lexer_returns_hash
    result = CodingAdventures::CSharpLexer.create_csharp_lexer("class Foo { }")
    assert_instance_of Hash, result
  end

  def test_create_csharp_lexer_stores_source
    result = CodingAdventures::CSharpLexer.create_csharp_lexer("class Foo { }")
    assert_equal "class Foo { }", result[:source]
  end

  def test_create_csharp_lexer_stores_nil_version
    result = CodingAdventures::CSharpLexer.create_csharp_lexer("class Foo { }")
    assert_nil result[:version]
  end

  def test_create_csharp_lexer_stores_language
    result = CodingAdventures::CSharpLexer.create_csharp_lexer("class Foo { }")
    assert_equal :csharp, result[:language]
  end

  def test_create_csharp_lexer_with_version
    result = CodingAdventures::CSharpLexer.create_csharp_lexer("class Foo { }", version: "8.0")
    assert_equal "8.0", result[:version]
  end

  def test_create_csharp_lexer_raises_for_unknown_version
    assert_raises(ArgumentError) do
      CodingAdventures::CSharpLexer.create_csharp_lexer("class Foo { }", version: "bogus")
    end
  end
end
