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

  # -----------------------------------------------------------------------
  # Extended format: mode directive
  # -----------------------------------------------------------------------

  def test_parse_mode_directive
    source = <<~TOKENS
      mode: indentation
      NAME = /[a-z]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal "indentation", grammar.mode
    assert_equal 1, grammar.definitions.length
  end

  def test_parse_mode_missing_value
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("mode:")
    end
    assert_includes error.message, "Missing value"
  end

  # -----------------------------------------------------------------------
  # Extended format: skip section
  # -----------------------------------------------------------------------

  def test_parse_skip_section
    source = <<~TOKENS
      NAME = /[a-z]+/
      skip:
        WHITESPACE = /[ \\t]+/
        COMMENT = /#[^\\n]*/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 1, grammar.definitions.length
    assert_equal 2, grammar.skip_definitions.length
    assert_equal "WHITESPACE", grammar.skip_definitions[0].name
    assert_equal "COMMENT", grammar.skip_definitions[1].name
  end

  def test_parse_skip_section_missing_equals
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("skip:\n  BAD_PATTERN")
    end
    assert_includes error.message, "Expected skip pattern"
  end

  def test_parse_skip_section_incomplete
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("skip:\n  BAD =")
    end
    assert_includes error.message, "Incomplete"
  end

  # -----------------------------------------------------------------------
  # Extended format: reserved section
  # -----------------------------------------------------------------------

  def test_parse_reserved_section
    source = <<~TOKENS
      NAME = /[a-z]+/
      reserved:
        class
        import
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal %w[class import], grammar.reserved_keywords
  end

  # -----------------------------------------------------------------------
  # Extended format: alias (-> TYPE)
  # -----------------------------------------------------------------------

  def test_parse_alias_regex
    source = 'STRING_DQ = /"[^"]*"/ -> STRING'
    grammar = GT.parse_token_grammar(source)
    defn = grammar.definitions[0]
    assert_equal "STRING_DQ", defn.name
    assert_equal "STRING", defn.alias_name
  end

  def test_parse_alias_literal
    source = 'PLUS_SIGN = "+" -> PLUS'
    grammar = GT.parse_token_grammar(source)
    defn = grammar.definitions[0]
    assert_equal "PLUS_SIGN", defn.name
    assert_equal "PLUS", defn.alias_name
  end

  def test_parse_alias_missing
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar('FOO = /x/ ->')
    end
    assert_includes error.message, "Missing alias"
  end

  def test_parse_alias_unexpected_text
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar('FOO = /x/ blah')
    end
    assert_includes error.message, "Unexpected text"
  end

  def test_parse_unclosed_regex
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar('FOO = /unclosed')
    end
    assert_includes error.message, "Unclosed regex"
  end

  def test_parse_unclosed_literal
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar('FOO = "unclosed')
    end
    assert_includes error.message, "Unclosed literal"
  end

  def test_parse_literal_alias_unexpected_text
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar('FOO = "x" blah')
    end
    assert_includes error.message, "Unexpected text"
  end

  def test_parse_literal_alias_missing
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar('FOO = "x" ->')
    end
    assert_includes error.message, "Missing alias"
  end

  # -----------------------------------------------------------------------
  # Token names with aliases
  # -----------------------------------------------------------------------

  def test_token_names_includes_aliases
    source = 'STRING_DQ = /"[^"]*"/ -> STRING'
    grammar = GT.parse_token_grammar(source)
    names = grammar.token_names
    assert_includes names, "STRING_DQ"
    assert_includes names, "STRING"
  end

  def test_effective_token_names_uses_alias
    source = <<~TOKENS
      NAME = /[a-z]+/
      STRING_DQ = /"[^"]*"/ -> STRING
    TOKENS
    grammar = GT.parse_token_grammar(source)
    effective = grammar.effective_token_names
    assert_includes effective, "NAME"
    assert_includes effective, "STRING"
    refute_includes effective, "STRING_DQ"
  end

  # -----------------------------------------------------------------------
  # Validation extensions
  # -----------------------------------------------------------------------

  def test_validate_unknown_mode
    grammar = GT::TokenGrammar.new(mode: "unknown")
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("Unknown lexer mode") }
  end

  def test_validate_indentation_mode_ok
    grammar = GT::TokenGrammar.new(mode: "indentation")
    issues = GT.validate_token_grammar(grammar)
    refute issues.any? { |i| i.include?("Unknown lexer mode") }
  end

  def test_validate_skip_definitions
    defn = GT::TokenDefinition.new(
      name: "BAD", pattern: "[invalid", is_regex: true,
      line_number: 1, alias_name: nil
    )
    grammar = GT::TokenGrammar.new(skip_definitions: [defn])
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("Invalid regex") }
  end

  def test_validate_alias_convention
    defn = GT::TokenDefinition.new(
      name: "FOO", pattern: "x", is_regex: false,
      line_number: 1, alias_name: "lowercase"
    )
    grammar = GT::TokenGrammar.new(definitions: [defn])
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("UPPER_CASE") && i.include?("Alias") }
  end

  # -----------------------------------------------------------------------
  # Starlark .tokens integration
  # -----------------------------------------------------------------------

  def test_parse_real_starlark_tokens
    path = File.join(__dir__, "..", "..", "..", "..", "..", "grammars", "starlark.tokens")
    skip("starlark.tokens not found") unless File.exist?(path)
    source = File.read(path)
    grammar = GT.parse_token_grammar(source)
    assert_equal "indentation", grammar.mode
    assert grammar.definitions.length > 10
    assert grammar.skip_definitions.length >= 1
    assert grammar.reserved_keywords.include?("class")
  end

  # -----------------------------------------------------------------------
  # Real grammar files (original tests)
  # -----------------------------------------------------------------------

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
