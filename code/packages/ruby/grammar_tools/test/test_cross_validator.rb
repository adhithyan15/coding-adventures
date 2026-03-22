# frozen_string_literal: true

require_relative "test_helper"

class TestCrossValidator < Minitest::Test
  GT = CodingAdventures::GrammarTools

  def test_fully_consistent
    token_source = <<~TOKENS
      NUMBER = /[0-9]+/
      PLUS   = "+"
    TOKENS
    grammar_source = <<~GRAMMAR
      expression = NUMBER { PLUS NUMBER } ;
    GRAMMAR
    tg = GT.parse_token_grammar(token_source)
    pg = GT.parse_parser_grammar(grammar_source)
    issues = GT.cross_validate(tg, pg)
    assert_empty issues
  end

  def test_missing_token_reference
    token_source = 'NUMBER = /[0-9]+/'
    grammar_source = "expression = NUMBER PLUS NUMBER ;"
    tg = GT.parse_token_grammar(token_source)
    pg = GT.parse_parser_grammar(grammar_source)
    issues = GT.cross_validate(tg, pg)
    assert issues.any? { |i| i.include?("Error") && i.include?("PLUS") }
  end

  def test_unused_token_warning
    token_source = <<~TOKENS
      NUMBER = /[0-9]+/
      PLUS   = "+"
      MINUS  = "-"
    TOKENS
    grammar_source = "expression = NUMBER { PLUS NUMBER } ;"
    tg = GT.parse_token_grammar(token_source)
    pg = GT.parse_parser_grammar(grammar_source)
    issues = GT.cross_validate(tg, pg)
    assert issues.any? { |i| i.include?("Warning") && i.include?("MINUS") }
  end

  def test_multiple_missing_and_unused
    token_source = <<~TOKENS
      NUMBER = /[0-9]+/
      TILDE  = "~"
    TOKENS
    grammar_source = "expression = NUMBER PLUS STAR NUMBER ;"
    tg = GT.parse_token_grammar(token_source)
    pg = GT.parse_parser_grammar(grammar_source)
    issues = GT.cross_validate(tg, pg)
    # Should have errors for PLUS and STAR, warning for TILDE
    errors = issues.select { |i| i.start_with?("Error") }
    warnings = issues.select { |i| i.start_with?("Warning") }
    assert_equal 2, errors.length
    assert_equal 1, warnings.length
  end

  # -----------------------------------------------------------------------
  # Indentation mode (implicit INDENT, DEDENT, NEWLINE)
  # -----------------------------------------------------------------------

  def test_indentation_mode_implicit_tokens
    token_source = <<~TOKENS
      mode: indentation
      NAME = /[a-z]+/
    TOKENS
    grammar_source = "file = { NAME NEWLINE } ;"
    tg = GT.parse_token_grammar(token_source)
    pg = GT.parse_parser_grammar(grammar_source)
    issues = GT.cross_validate(tg, pg)
    # NEWLINE should be implicitly available
    errors = issues.select { |i| i.start_with?("Error") }
    assert_empty errors
  end

  def test_indent_dedent_without_indentation_mode
    # Without indentation mode, INDENT/DEDENT are errors but NEWLINE is valid.
    #
    # NEWLINE is always a valid synthetic token because the lexer emits it
    # whenever a bare newline is encountered and no skip pattern consumed it.
    # INDENT/DEDENT are only valid in indentation mode.
    token_source = 'NAME = /[a-z]+/'
    grammar_source = "file = NAME NEWLINE INDENT NAME DEDENT ;"
    tg = GT.parse_token_grammar(token_source)
    pg = GT.parse_parser_grammar(grammar_source)
    issues = GT.cross_validate(tg, pg)
    errors = issues.select { |i| i.start_with?("Error") }
    assert errors.any? { |i| i.include?("INDENT") }
    assert errors.any? { |i| i.include?("DEDENT") }
    refute errors.any? { |i| i.include?("NEWLINE") }
  end

  def test_eof_always_implicit
    token_source = 'NAME = /[a-z]+/'
    grammar_source = "file = NAME EOF ;"
    tg = GT.parse_token_grammar(token_source)
    pg = GT.parse_parser_grammar(grammar_source)
    issues = GT.cross_validate(tg, pg)
    errors = issues.select { |i| i.start_with?("Error") }
    refute errors.any? { |i| i.include?("EOF") }
  end

  # -----------------------------------------------------------------------
  # Alias-based "used" detection
  # -----------------------------------------------------------------------

  def test_alias_counts_as_used
    token_source = 'STRING_DQ = /"[^"]*"/ -> STRING'
    grammar_source = "expr = STRING ;"
    tg = GT.parse_token_grammar(token_source)
    pg = GT.parse_parser_grammar(grammar_source)
    issues = GT.cross_validate(tg, pg)
    # STRING_DQ should not trigger "unused" warning because STRING is referenced
    warnings = issues.select { |i| i.start_with?("Warning") }
    refute warnings.any? { |i| i.include?("STRING_DQ") }
  end

  def test_unreferenced_alias_warns
    token_source = 'STRING_DQ = /"[^"]*"/ -> STRING'
    grammar_source = "expr = NAME ;"
    tg = GT.parse_token_grammar(token_source)
    pg = GT.parse_parser_grammar(grammar_source)
    issues = GT.cross_validate(tg, pg)
    warnings = issues.select { |i| i.start_with?("Warning") }
    assert warnings.any? { |i| i.include?("STRING_DQ") }
  end

  # -----------------------------------------------------------------------
  # Real grammar cross-validation (original tests)
  # -----------------------------------------------------------------------

  def test_real_python_cross_validation
    tokens_path = File.join(__dir__, "..", "..", "..", "..", "..", "grammars", "python.tokens")
    grammar_path = File.join(__dir__, "..", "..", "..", "..", "..", "grammars", "python.grammar")
    skip("Grammar files not found") unless File.exist?(tokens_path) && File.exist?(grammar_path)
    tg = GT.parse_token_grammar(File.read(tokens_path))
    pg = GT.parse_parser_grammar(File.read(grammar_path))
    issues = GT.cross_validate(tg, pg)
    errors = issues.select { |i| i.start_with?("Error") }
    # No missing token references in a well-formed grammar pair.
    assert_empty errors
  end
end
