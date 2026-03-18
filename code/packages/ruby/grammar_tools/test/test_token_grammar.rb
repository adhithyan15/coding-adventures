# frozen_string_literal: true

require_relative "test_helper"

class TestTokenGrammar < Minitest::Test
  GT = CodingAdventures::GrammarTools

  # -----------------------------------------------------------------------
  # Parsing basics
  # -----------------------------------------------------------------------

  def test_parse_regex_token
    source = 'NUMBER = /[0-9]+/'
    grammar = GT.parse_token_grammar(source)
    assert_equal 1, grammar.definitions.length
    defn = grammar.definitions[0]
    assert_equal "NUMBER", defn.name
    assert_equal "[0-9]+", defn.pattern
    assert defn.is_regex
    assert_equal 1, defn.line_number
  end

  def test_parse_literal_token
    source = 'PLUS = "+"'
    grammar = GT.parse_token_grammar(source)
    assert_equal 1, grammar.definitions.length
    defn = grammar.definitions[0]
    assert_equal "PLUS", defn.name
    assert_equal "+", defn.pattern
    refute defn.is_regex
  end

  def test_parse_multiple_definitions
    source = <<~TOKENS
      NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
      NUMBER = /[0-9]+/
      PLUS   = "+"
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 3, grammar.definitions.length
    assert_equal %w[NAME NUMBER PLUS], grammar.definitions.map(&:name)
  end

  def test_parse_keywords_section
    source = <<~TOKENS
      NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

      keywords:
        if
        else
        while
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 1, grammar.definitions.length
    assert_equal %w[if else while], grammar.keywords
  end

  def test_parse_keywords_with_tab_indent
    source = "NAME = /[a-z]+/\nkeywords:\n\tif\n\telse"
    grammar = GT.parse_token_grammar(source)
    assert_equal %w[if else], grammar.keywords
  end

  def test_parse_comments_and_blanks
    source = <<~TOKENS
      # This is a comment
      NAME = /[a-zA-Z]+/

      # Another comment
      NUMBER = /[0-9]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 2, grammar.definitions.length
  end

  def test_token_names_set
    source = "NAME = /[a-z]+/\nNUMBER = /[0-9]+/"
    grammar = GT.parse_token_grammar(source)
    assert_equal Set.new(%w[NAME NUMBER]), grammar.token_names
  end

  def test_keywords_colon_with_space
    source = "NAME = /[a-z]+/\nkeywords :\n  if"
    grammar = GT.parse_token_grammar(source)
    assert_equal ["if"], grammar.keywords
  end

  def test_definitions_after_keywords
    source = <<~TOKENS
      NAME = /[a-z]+/
      keywords:
        if
      NUMBER = /[0-9]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 2, grammar.definitions.length
    assert_equal ["if"], grammar.keywords
  end

  # -----------------------------------------------------------------------
  # Parsing errors
  # -----------------------------------------------------------------------

  def test_error_missing_equals
    assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("NOT_A_DEFINITION")
    end
  end

  def test_error_missing_name
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar('= /pattern/')
    end
    assert_includes error.message, "Missing token name"
  end

  def test_error_invalid_name
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar('123BAD = /pattern/')
    end
    assert_includes error.message, "Invalid token name"
  end

  def test_error_missing_pattern
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("NAME =")
    end
    assert_includes error.message, "Missing pattern"
  end

  def test_error_empty_regex
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("NAME = //")
    end
    assert_includes error.message, "Empty regex"
  end

  def test_error_empty_literal
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar('NAME = ""')
    end
    assert_includes error.message, "Empty literal"
  end

  def test_error_bad_pattern_format
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("NAME = nope")
    end
    assert_includes error.message, "must be /regex/"
  end

  # -----------------------------------------------------------------------
  # Validation
  # -----------------------------------------------------------------------

  def test_validate_duplicate_names
    source = "FOO = /a/\nFOO = /b/"
    grammar = GT.parse_token_grammar(source)
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("Duplicate") }
  end

  def test_validate_invalid_regex
    # Force a bad regex by manually constructing
    defn = GT::TokenDefinition.new(name: "BAD", pattern: "[invalid", is_regex: true, line_number: 1)
    grammar = GT::TokenGrammar.new(definitions: [defn])
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("Invalid regex") }
  end

  def test_validate_non_uppercase
    defn = GT::TokenDefinition.new(name: "lowercase", pattern: "x", is_regex: false, line_number: 1)
    grammar = GT::TokenGrammar.new(definitions: [defn])
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("UPPER_CASE") }
  end

  def test_validate_empty_pattern_warning
    defn = GT::TokenDefinition.new(name: "BAD", pattern: "", is_regex: false, line_number: 1)
    grammar = GT::TokenGrammar.new(definitions: [defn])
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("Empty pattern") }
  end

  def test_validate_clean_grammar
    source = 'NUMBER = /[0-9]+/'
    grammar = GT.parse_token_grammar(source)
    issues = GT.validate_token_grammar(grammar)
    assert_empty issues
  end

  # -----------------------------------------------------------------------
  # Real grammar files
  # -----------------------------------------------------------------------

  def test_parse_real_python_tokens
    path = File.join(__dir__, "..", "..", "..", "..", "..", "grammars", "python.tokens")
    skip("python.tokens not found") unless File.exist?(path)
    source = File.read(path)
    grammar = GT.parse_token_grammar(source)
    assert grammar.definitions.length > 5
    assert grammar.keywords.include?("if")
    assert grammar.token_names.include?("NUMBER")
  end

  def test_parse_real_ruby_tokens
    path = File.join(__dir__, "..", "..", "..", "..", "..", "grammars", "ruby.tokens")
    skip("ruby.tokens not found") unless File.exist?(path)
    source = File.read(path)
    grammar = GT.parse_token_grammar(source)
    assert grammar.definitions.length > 5
    assert grammar.keywords.include?("def")
    assert grammar.keywords.include?("end")
  end
end
