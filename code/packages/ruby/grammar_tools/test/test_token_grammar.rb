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

  def test_parse_layout_mode_directive
    source = <<~TOKENS
      mode: layout
      NAME = /[a-z]+/
      layout_keywords:
        let
        where
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal "layout", grammar.mode
    assert_equal %w[let where], grammar.layout_keywords
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

  def test_validate_layout_mode_requires_layout_keywords
    grammar = GT::TokenGrammar.new(mode: "layout")
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("layout_keywords") }
  end

  def test_validate_layout_mode_ok
    grammar = GT::TokenGrammar.new(mode: "layout", layout_keywords: ["let"])
    issues = GT.validate_token_grammar(grammar)
    refute issues.any? { |i| i.include?("Unknown lexer mode") }
    refute issues.any? { |i| i.include?("layout_keywords") }
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

  # -----------------------------------------------------------------------
  # Pattern groups: parsing
  # -----------------------------------------------------------------------

  def test_basic_group
    source = <<~TOKENS
      TEXT = /[^<]+/
      TAG_OPEN = "<"

      group tag:
        TAG_NAME = /[a-zA-Z]+/
        TAG_CLOSE = ">"
    TOKENS
    grammar = GT.parse_token_grammar(source)

    # Default group patterns
    assert_equal 2, grammar.definitions.length
    assert_equal "TEXT", grammar.definitions[0].name
    assert_equal "TAG_OPEN", grammar.definitions[1].name

    # Named group
    assert grammar.groups.key?("tag")
    group = grammar.groups["tag"]
    assert_instance_of GT::PatternGroup, group
    assert_equal "tag", group.name
    assert_equal 2, group.definitions.length
    assert_equal "TAG_NAME", group.definitions[0].name
    assert_equal "TAG_CLOSE", group.definitions[1].name
  end

  def test_multiple_groups
    source = <<~TOKENS
      TEXT = /[^<]+/

      group tag:
        TAG_NAME = /[a-zA-Z]+/

      group cdata:
        CDATA_TEXT = /[^]]+/
        CDATA_END = "]]>"
    TOKENS
    grammar = GT.parse_token_grammar(source)

    assert_equal 2, grammar.groups.length
    assert grammar.groups.key?("tag")
    assert grammar.groups.key?("cdata")
    assert_equal 1, grammar.groups["tag"].definitions.length
    assert_equal 2, grammar.groups["cdata"].definitions.length
  end

  def test_group_with_alias
    source = <<~TOKENS
      TEXT = /[^<]+/

      group tag:
        ATTR_VALUE_DQ = /"[^"]*"/ -> ATTR_VALUE
        ATTR_VALUE_SQ = /'[^']*'/ -> ATTR_VALUE
    TOKENS
    grammar = GT.parse_token_grammar(source)

    group = grammar.groups["tag"]
    assert_equal "ATTR_VALUE_DQ", group.definitions[0].name
    assert_equal "ATTR_VALUE", group.definitions[0].alias_name
    assert_equal "ATTR_VALUE_SQ", group.definitions[1].name
    assert_equal "ATTR_VALUE", group.definitions[1].alias_name
  end

  def test_group_with_literal_patterns
    source = <<~TOKENS
      TEXT = /[^<]+/

      group tag:
        EQUALS = "="
        TAG_NAME = /[a-zA-Z]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)

    group = grammar.groups["tag"]
    refute group.definitions[0].is_regex
    assert_equal "=", group.definitions[0].pattern
    assert group.definitions[1].is_regex
  end

  def test_no_groups_backward_compat
    source = "NUMBER = /[0-9]+/\nPLUS = \"+\"\n"
    grammar = GT.parse_token_grammar(source)

    assert_equal({}, grammar.groups)
    assert_equal 2, grammar.definitions.length
  end

  def test_groups_with_skip_section
    source = <<~TOKENS
      skip:
        WS = /[ \\t]+/

      TEXT = /[^<]+/

      group tag:
        TAG_NAME = /[a-zA-Z]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)

    assert_equal 1, grammar.skip_definitions.length
    assert_equal 1, grammar.definitions.length
    assert_equal 1, grammar.groups.length
  end

  def test_token_names_includes_groups
    source = <<~TOKENS
      TEXT = /[^<]+/

      group tag:
        TAG_NAME = /[a-zA-Z]+/
        ATTR_DQ = /"[^"]*"/ -> ATTR_VALUE
    TOKENS
    grammar = GT.parse_token_grammar(source)

    names = grammar.token_names
    assert_includes names, "TEXT"
    assert_includes names, "TAG_NAME"
    assert_includes names, "ATTR_DQ"
    assert_includes names, "ATTR_VALUE"
  end

  def test_effective_token_names_includes_groups
    source = <<~TOKENS
      TEXT = /[^<]+/

      group tag:
        ATTR_DQ = /"[^"]*"/ -> ATTR_VALUE
    TOKENS
    grammar = GT.parse_token_grammar(source)

    names = grammar.effective_token_names
    assert_includes names, "TEXT"
    assert_includes names, "ATTR_VALUE"
    refute_includes names, "ATTR_DQ" # alias replaces name
  end

  # -----------------------------------------------------------------------
  # Pattern groups: validation
  # -----------------------------------------------------------------------

  def test_group_validates_definitions
    grammar = GT::TokenGrammar.new(
      groups: {
        "tag" => GT::PatternGroup.new(
          name: "tag",
          definitions: [
            GT::TokenDefinition.new(
              name: "BAD", pattern: "[invalid",
              is_regex: true, line_number: 5
            )
          ]
        )
      }
    )
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("Invalid regex") }
  end

  def test_empty_group_warning
    grammar = GT::TokenGrammar.new(
      groups: {
        "empty" => GT::PatternGroup.new(name: "empty", definitions: [])
      }
    )
    issues = GT.validate_token_grammar(grammar)
    assert issues.any? { |i| i.include?("Empty pattern group") }
  end

  # -----------------------------------------------------------------------
  # Pattern groups: error handling
  # -----------------------------------------------------------------------

  def test_error_missing_group_name
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("TEXT = /abc/\ngroup :\n  FOO = /x/\n")
    end
    assert_includes error.message, "Missing group name"
  end

  def test_error_invalid_group_name_uppercase
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("TEXT = /abc/\ngroup Tag:\n  FOO = /x/\n")
    end
    assert_includes error.message, "Invalid group name"
  end

  def test_error_invalid_group_name_starts_with_digit
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("TEXT = /abc/\ngroup 1tag:\n  FOO = /x/\n")
    end
    assert_includes error.message, "Invalid group name"
  end

  def test_error_reserved_group_name_default
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("TEXT = /abc/\ngroup default:\n  FOO = /x/\n")
    end
    assert_includes error.message, "Reserved group name"
  end

  def test_error_reserved_group_name_skip
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("TEXT = /abc/\ngroup skip:\n  FOO = /x/\n")
    end
    assert_includes error.message, "Reserved group name"
  end

  def test_error_reserved_group_name_keywords
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("TEXT = /abc/\ngroup keywords:\n  FOO = /x/\n")
    end
    assert_includes error.message, "Reserved group name"
  end

  def test_error_duplicate_group_name
    source = "TEXT = /abc/\ngroup tag:\n  FOO = /x/\ngroup tag:\n  BAR = /y/\n"
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar(source)
    end
    assert_includes error.message, "Duplicate group name"
  end

  def test_error_bad_definition_in_group
    source = "TEXT = /abc/\ngroup tag:\n  not a definition\n"
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar(source)
    end
    assert_includes error.message, "Expected token definition"
  end

  def test_error_incomplete_definition_in_group
    source = "TEXT = /abc/\ngroup tag:\n  FOO = \n"
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar(source)
    end
    assert_includes error.message, "Incomplete definition"
  end

  # -----------------------------------------------------------------------
  # Magic comments
  # -----------------------------------------------------------------------

  def test_magic_version_sets_version
    # A "# @version N" line should set grammar.version to N.
    source = <<~TOKENS
      # @version 1
      NUMBER = /[0-9]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 1, grammar.version
  end

  def test_magic_version_default_is_zero
    # When no @version comment is present the default must be 0 so that
    # existing grammar files remain valid without any changes.
    grammar = GT.parse_token_grammar("NUMBER = /[0-9]+/")
    assert_equal 0, grammar.version
  end

  def test_magic_case_insensitive_true
    # "# @case_insensitive true" should set case_insensitive to true.
    source = <<~TOKENS
      # @case_insensitive true
      NAME = /[a-z]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert grammar.case_insensitive
  end

  def test_magic_case_insensitive_false
    # "# @case_insensitive false" should set case_insensitive to false
    # (explicit opt-out is still valid).
    source = <<~TOKENS
      # @case_insensitive false
      NAME = /[a-z]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    refute grammar.case_insensitive
  end

  def test_magic_case_insensitive_default_is_false
    # When the magic comment is absent, case_insensitive must default to false
    # so that existing grammar files are unaffected.
    grammar = GT.parse_token_grammar("NUMBER = /[0-9]+/")
    refute grammar.case_insensitive
  end

  def test_magic_unknown_key_silently_ignored
    # An unknown @key must not raise an error; it is simply ignored.
    # This allows future extensions without breaking older parsers.
    source = <<~TOKENS
      # @unknown_key some_value
      NUMBER = /[0-9]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 1, grammar.definitions.length
  end

  def test_magic_both_comments_together
    # Both @version and @case_insensitive can appear in the same file.
    source = <<~TOKENS
      # @version 3
      # @case_insensitive true
      NAME = /[a-z]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 3, grammar.version
    assert grammar.case_insensitive
  end

  def test_magic_version_with_surrounding_content
    # Magic comments should work regardless of where they appear in the file
    # (before, between, or after token definitions).
    source = <<~TOKENS
      NUMBER = /[0-9]+/
      # @version 2
      NAME = /[a-z]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 2, grammar.version
    assert_equal 2, grammar.definitions.length
  end

  def test_magic_plain_comment_still_ignored
    # A plain comment (no @key) must still be skipped without any side-effects.
    source = <<~TOKENS
      # just a plain comment
      NUMBER = /[0-9]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 0, grammar.version
    refute grammar.case_insensitive
    assert_equal 1, grammar.definitions.length
  end

  # -----------------------------------------------------------------------
  # soft_keywords section
  # -----------------------------------------------------------------------

  def test_parse_soft_keywords_section
    source = <<~TOKENS
      NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

      soft_keywords:
        match
        case
        _
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 1, grammar.definitions.length
    assert_equal %w[match case _], grammar.soft_keywords
  end

  def test_parse_soft_keywords_with_space_colon
    source = "NAME = /[a-z]+/\nsoft_keywords :\n  match\n  case"
    grammar = GT.parse_token_grammar(source)
    assert_equal %w[match case], grammar.soft_keywords
  end

  def test_soft_keywords_default_empty
    grammar = GT.parse_token_grammar("NUMBER = /[0-9]+/")
    assert_equal [], grammar.soft_keywords
  end

  def test_soft_keywords_with_context_keywords
    # Both context_keywords and soft_keywords can coexist in the same file.
    source = <<~TOKENS
      NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

      context_keywords:
        async
        await

      soft_keywords:
        match
        case
        type
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal %w[async await], grammar.context_keywords
    assert_equal %w[match case type], grammar.soft_keywords
  end

  def test_definitions_after_soft_keywords
    source = <<~TOKENS
      NAME = /[a-z]+/
      soft_keywords:
        match
      NUMBER = /[0-9]+/
    TOKENS
    grammar = GT.parse_token_grammar(source)
    assert_equal 2, grammar.definitions.length
    assert_equal %w[match], grammar.soft_keywords
  end

  def test_soft_keywords_reserved_group_name
    # "soft_keywords" should be a reserved group name, preventing
    # a pattern group from using it.
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("TEXT = /abc/\ngroup soft_keywords:\n  FOO = /x/\n")
    end
    assert_includes error.message, "Reserved group name"
  end

  def test_context_keywords_reserved_group_name
    # "context_keywords" should also be a reserved group name.
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("TEXT = /abc/\ngroup context_keywords:\n  FOO = /x/\n")
    end
    assert_includes error.message, "Reserved group name"
  end

  def test_layout_keywords_reserved_group_name
    error = assert_raises(GT::TokenGrammarError) do
      GT.parse_token_grammar("TEXT = /abc/\ngroup layout_keywords:\n  FOO = /x/\n")
    end
    assert_includes error.message, "Reserved group name"
  end

  def test_parse_real_python312_tokens_soft_keywords
    path = File.join(__dir__, "..", "..", "..", "..", "..", "grammars", "python", "python3.12.tokens")
    skip("python3.12.tokens not found") unless File.exist?(path)
    source = File.read(path)
    grammar = GT.parse_token_grammar(source)
    assert grammar.soft_keywords.length > 0, "python3.12.tokens should have soft_keywords"
    assert grammar.soft_keywords.include?("match"), "Expected 'match' in soft_keywords"
    assert grammar.soft_keywords.include?("case"), "Expected 'case' in soft_keywords"
  end
end
