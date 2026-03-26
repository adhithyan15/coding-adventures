# frozen_string_literal: true

require_relative "test_helper"

class TestParserGrammar < Minitest::Test
  GT = CodingAdventures::GrammarTools

  # -----------------------------------------------------------------------
  # Parsing basics
  # -----------------------------------------------------------------------

  def test_parse_simple_rule
    source = "factor = NUMBER ;"
    grammar = GT.parse_parser_grammar(source)
    assert_equal 1, grammar.rules.length
    assert_equal "factor", grammar.rules[0].name
    rule = grammar.rules[0]
    assert_kind_of GT::RuleReference, rule.body
    assert_equal "NUMBER", rule.body.name
    assert rule.body.is_token
  end

  def test_parse_alternation
    source = "factor = NUMBER | STRING ;"
    grammar = GT.parse_parser_grammar(source)
    body = grammar.rules[0].body
    assert_kind_of GT::Alternation, body
    assert_equal 2, body.choices.length
  end

  def test_parse_sequence
    source = "assignment = NAME EQUALS expression ;"
    grammar = GT.parse_parser_grammar(source)
    body = grammar.rules[0].body
    assert_kind_of GT::Sequence, body
    assert_equal 3, body.elements.length
  end

  def test_parse_repetition
    source = "program = { statement } ;"
    grammar = GT.parse_parser_grammar(source)
    body = grammar.rules[0].body
    assert_kind_of GT::Repetition, body
  end

  def test_parse_optional
    source = "maybe = [ NAME ] ;"
    grammar = GT.parse_parser_grammar(source)
    body = grammar.rules[0].body
    assert_kind_of GT::OptionalElement, body
  end

  def test_parse_group
    source = "expr = term { ( PLUS | MINUS ) term } ;"
    grammar = GT.parse_parser_grammar(source)
    body = grammar.rules[0].body
    # Should be a Sequence of [term, Repetition]
    assert_kind_of GT::Sequence, body
  end

  def test_parse_literal
    source = 'op = "++" ;'
    grammar = GT.parse_parser_grammar(source)
    body = grammar.rules[0].body
    assert_kind_of GT::Literal, body
    assert_equal "++", body.value
  end

  def test_parse_lowercase_rule_reference
    source = "program = expression ;"
    grammar = GT.parse_parser_grammar(source)
    body = grammar.rules[0].body
    assert_kind_of GT::RuleReference, body
    refute body.is_token
    assert_equal "expression", body.name
  end

  def test_parse_multiple_rules
    source = <<~GRAMMAR
      program = { statement } ;
      statement = assignment ;
      assignment = NAME EQUALS NUMBER ;
    GRAMMAR
    grammar = GT.parse_parser_grammar(source)
    assert_equal 3, grammar.rules.length
    assert_equal %w[program statement assignment], grammar.rules.map(&:name)
  end

  def test_rule_names_set
    source = "a = NUMBER ;\nb = STRING ;"
    grammar = GT.parse_parser_grammar(source)
    assert_equal Set.new(%w[a b]), grammar.rule_names
  end

  def test_token_references
    source = "expr = NUMBER PLUS STRING ;"
    grammar = GT.parse_parser_grammar(source)
    refs = grammar.token_references
    assert_includes refs, "NUMBER"
    assert_includes refs, "PLUS"
    assert_includes refs, "STRING"
  end

  def test_rule_references
    source = "program = expression ;\nexpression = NUMBER ;"
    grammar = GT.parse_parser_grammar(source)
    refs = grammar.rule_references
    assert_includes refs, "expression"
  end

  def test_line_numbers
    source = "a = NUMBER ;\n\nb = STRING ;"
    grammar = GT.parse_parser_grammar(source)
    assert_equal 1, grammar.rules[0].line_number
    assert_equal 3, grammar.rules[1].line_number
  end

  # -----------------------------------------------------------------------
  # Parsing with comments
  # -----------------------------------------------------------------------

  def test_comments_are_skipped
    source = <<~GRAMMAR
      # This is a comment
      program = { statement } ;
      # Another comment
      statement = NUMBER ;
    GRAMMAR
    grammar = GT.parse_parser_grammar(source)
    assert_equal 2, grammar.rules.length
  end

  # -----------------------------------------------------------------------
  # Parsing errors
  # -----------------------------------------------------------------------

  def test_error_unexpected_character
    assert_raises(GT::ParserGrammarError) do
      GT.parse_parser_grammar("rule = @ ;")
    end
  end

  def test_error_missing_semicolon
    assert_raises(GT::ParserGrammarError) do
      GT.parse_parser_grammar("rule = NUMBER")
    end
  end

  def test_error_unterminated_string
    assert_raises(GT::ParserGrammarError) do
      GT.parse_parser_grammar('rule = "unterminated ;')
    end
  end

  def test_error_unexpected_token
    assert_raises(GT::ParserGrammarError) do
      GT.parse_parser_grammar("rule = ; ;")
    end
  end

  def test_error_missing_rule_name
    assert_raises(GT::ParserGrammarError) do
      GT.parse_parser_grammar("= NUMBER ;")
    end
  end

  # -----------------------------------------------------------------------
  # Validation
  # -----------------------------------------------------------------------

  def test_validate_undefined_rule_ref
    source = "program = nonexistent ;"
    grammar = GT.parse_parser_grammar(source)
    issues = GT.validate_parser_grammar(grammar)
    assert issues.any? { |i| i.include?("Undefined rule") }
  end

  def test_validate_undefined_token_ref
    source = "program = BOGUS ;"
    grammar = GT.parse_parser_grammar(source)
    issues = GT.validate_parser_grammar(grammar, token_names: Set.new(%w[NUMBER]))
    assert issues.any? { |i| i.include?("Undefined token") }
  end

  def test_validate_duplicate_rules
    source = "a = NUMBER ;\na = STRING ;"
    grammar = GT.parse_parser_grammar(source)
    issues = GT.validate_parser_grammar(grammar)
    assert issues.any? { |i| i.include?("Duplicate") }
  end

  def test_validate_unreachable_rule
    source = "a = NUMBER ;\nb = STRING ;"
    grammar = GT.parse_parser_grammar(source)
    issues = GT.validate_parser_grammar(grammar)
    assert issues.any? { |i| i.include?("unreachable") }
  end

  def test_validate_non_lowercase_rule
    source = "MyRule = NUMBER ;"
    grammar = GT.parse_parser_grammar(source)
    issues = GT.validate_parser_grammar(grammar)
    assert issues.any? { |i| i.include?("lowercase") }
  end

  def test_validate_clean_grammar
    source = <<~GRAMMAR
      program = { statement } ;
      statement = NUMBER ;
    GRAMMAR
    grammar = GT.parse_parser_grammar(source)
    issues = GT.validate_parser_grammar(grammar)
    assert_empty issues
  end

  # -----------------------------------------------------------------------
  # Real grammar files
  # -----------------------------------------------------------------------

  def test_parse_real_python_grammar
    path = File.join(__dir__, "..", "..", "..", "..", "..", "grammars", "python.grammar")
    skip("python.grammar not found") unless File.exist?(path)
    source = File.read(path)
    grammar = GT.parse_parser_grammar(source)
    assert grammar.rules.length > 3
    assert_equal "program", grammar.rules[0].name
  end

  def test_parse_real_ruby_grammar
    path = File.join(__dir__, "..", "..", "..", "..", "..", "grammars", "ruby.grammar")
    skip("ruby.grammar not found") unless File.exist?(path)
    source = File.read(path)
    grammar = GT.parse_parser_grammar(source)
    assert grammar.rules.length > 5
    assert_equal "program", grammar.rules[0].name
  end

  # -----------------------------------------------------------------------
  # Magic comments
  # -----------------------------------------------------------------------

  def test_magic_version_sets_version
    # A "# @version N" magic comment should set grammar.version to N.
    source = <<~GRAMMAR
      # @version 1
      program = { statement } ;
      statement = NUMBER ;
    GRAMMAR
    grammar = GT.parse_parser_grammar(source)
    assert_equal 1, grammar.version
    # Rules must still be parsed correctly.
    assert_equal 2, grammar.rules.length
  end

  def test_magic_version_default_is_zero
    # When no @version comment is present the default must be 0 so that
    # existing grammar files remain valid without any changes.
    source = "factor = NUMBER ;"
    grammar = GT.parse_parser_grammar(source)
    assert_equal 0, grammar.version
  end

  def test_magic_unknown_key_silently_ignored
    # An unknown @key must not raise an error and must not affect the rules.
    source = <<~GRAMMAR
      # @future_option yes
      factor = NUMBER ;
    GRAMMAR
    grammar = GT.parse_parser_grammar(source)
    assert_equal 1, grammar.rules.length
    assert_equal 0, grammar.version
  end

  def test_magic_plain_comment_still_ignored
    # A plain comment (no @key) must continue to be silently ignored.
    source = <<~GRAMMAR
      # ordinary comment
      factor = NUMBER ;
    GRAMMAR
    grammar = GT.parse_parser_grammar(source)
    assert_equal 0, grammar.version
    assert_equal 1, grammar.rules.length
  end

  def test_magic_version_mid_file
    # @version may appear anywhere in the file, not just at the top.
    source = <<~GRAMMAR
      program = { statement } ;
      # @version 5
      statement = NUMBER ;
    GRAMMAR
    grammar = GT.parse_parser_grammar(source)
    assert_equal 5, grammar.version
    assert_equal 2, grammar.rules.length
  end
end
