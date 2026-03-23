# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# Tests for GrammarDrivenParser trace: keyword option
# ==========================================================================
#
# When trace: true is passed to GrammarDrivenParser, every call to parse_rule
# should emit a [TRACE] line to $stderr. These tests verify:
#
#   1. The parse result is identical with and without trace mode.
#   2. Trace output goes to $stderr (not stdout).
#   3. Each line follows the documented format:
#        [TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match|fail
#   4. Both match and fail outcomes appear for a grammar with alternation.
#   5. No trace output is emitted when trace: false (the default).
#
# The arrow character is the Unicode right arrow U+2192 (→), matching the
# spec exactly. Tests use string_include? rather than regex so the expected
# literal text is obvious at a glance.
# ==========================================================================
class TestTrace < Minitest::Test
  P = CodingAdventures::Parser
  GDP = P::GrammarDrivenParser
  GT = CodingAdventures::GrammarTools
  TT = CodingAdventures::Lexer::TokenType
  Tokenizer = CodingAdventures::Lexer::Tokenizer

  # A simple arithmetic grammar with alternation so we get both match and fail
  # outcomes in a single parse.
  GRAMMAR_SOURCE = <<~GRAMMAR
    program      = { statement } ;
    statement    = assignment | expression_stmt ;
    assignment   = NAME EQUALS expression ;
    expression_stmt = expression ;
    expression   = term { ( PLUS | MINUS ) term } ;
    term         = factor { ( STAR | SLASH ) factor } ;
    factor       = NUMBER | STRING | NAME | LPAREN expression RPAREN ;
  GRAMMAR

  def grammar
    @grammar ||= GT.parse_parser_grammar(GRAMMAR_SOURCE)
  end

  # Run the parser with trace: true and capture $stderr output.
  # Returns [ast, captured_stderr_lines].
  def parse_with_trace(source)
    tokens = Tokenizer.new(source).tokenize
    # Capture $stderr by temporarily replacing it with a StringIO.
    original_stderr = $stderr
    fake_stderr = StringIO.new
    $stderr = fake_stderr
    begin
      ast = GDP.new(tokens, grammar, trace: true).parse
    ensure
      $stderr = original_stderr
    end
    lines = fake_stderr.string.lines.map(&:chomp)
    [ast, lines]
  end

  # -------------------------------------------------------------------------
  # Correctness: trace mode must not change the parse result
  # -------------------------------------------------------------------------

  def test_trace_does_not_change_parse_result_for_number
    tokens = Tokenizer.new("42").tokenize
    ast_no_trace = GDP.new(tokens, grammar).parse
    ast_with_trace, = parse_with_trace("42")
    assert_equal ast_no_trace.rule_name, ast_with_trace.rule_name
    assert_equal ast_no_trace.children.length, ast_with_trace.children.length
  end

  def test_trace_does_not_change_parse_result_for_assignment
    tokens = Tokenizer.new("x = 42\n").tokenize
    ast_no_trace = GDP.new(tokens, grammar).parse
    ast_with_trace, = parse_with_trace("x = 42\n")
    assert_equal "program", ast_with_trace.rule_name
    assert_equal ast_no_trace.rule_name, ast_with_trace.rule_name
  end

  def test_trace_does_not_change_parse_result_for_expression
    ast, = parse_with_trace("1 + 2 * 3")
    assert_equal "program", ast.rule_name
  end

  # -------------------------------------------------------------------------
  # Trace lines are emitted to $stderr
  # -------------------------------------------------------------------------

  def test_trace_emits_lines
    _ast, lines = parse_with_trace("42")
    assert lines.length > 0, "Expected trace lines on $stderr, got none"
  end

  def test_no_trace_emits_nothing_by_default
    tokens = Tokenizer.new("42").tokenize
    original_stderr = $stderr
    fake_stderr = StringIO.new
    $stderr = fake_stderr
    begin
      GDP.new(tokens, grammar).parse
    ensure
      $stderr = original_stderr
    end
    assert_empty fake_stderr.string, "Expected no $stderr output when trace: false"
  end

  def test_explicit_trace_false_emits_nothing
    tokens = Tokenizer.new("42").tokenize
    original_stderr = $stderr
    fake_stderr = StringIO.new
    $stderr = fake_stderr
    begin
      GDP.new(tokens, grammar, trace: false).parse
    ensure
      $stderr = original_stderr
    end
    assert_empty fake_stderr.string, "Expected no $stderr output when trace: false"
  end

  # -------------------------------------------------------------------------
  # Trace line format
  # -------------------------------------------------------------------------

  def test_trace_lines_start_with_trace_tag
    _ast, lines = parse_with_trace("42")
    lines.each do |line|
      assert line.start_with?("[TRACE]"), "Expected line to start with [TRACE]: #{line.inspect}"
    end
  end

  def test_trace_lines_contain_rule_keyword
    _ast, lines = parse_with_trace("42")
    lines.each do |line|
      assert_includes line, "rule '", "Expected 'rule \\'' in: #{line.inspect}"
    end
  end

  def test_trace_lines_contain_at_token
    _ast, lines = parse_with_trace("42")
    lines.each do |line|
      assert_match(/at token \d+/, line, "Expected 'at token <N>' in: #{line.inspect}")
    end
  end

  def test_trace_lines_contain_unicode_arrow
    _ast, lines = parse_with_trace("42")
    lines.each do |line|
      assert_includes line, "\u2192", "Expected \u2192 arrow in: #{line.inspect}"
    end
  end

  def test_trace_lines_end_with_match_or_fail
    _ast, lines = parse_with_trace("42")
    lines.each do |line|
      outcome = line.split("\u2192").last.strip
      assert_includes %w[match fail], outcome,
        "Expected outcome to be 'match' or 'fail', got: #{outcome.inspect} in #{line.inspect}"
    end
  end

  # -------------------------------------------------------------------------
  # Both outcomes appear (grammar with alternation triggers fail branches)
  # -------------------------------------------------------------------------

  def test_trace_has_match_outcome
    # Parsing "42" goes through expression_stmt -> expression -> ... -> NUMBER
    # which all match.
    _ast, lines = parse_with_trace("42")
    has_match = lines.any? { |l| l.end_with?("match") }
    assert has_match, "Expected at least one trace line ending with 'match'"
  end

  def test_trace_has_fail_outcome
    # The grammar tries 'assignment' before 'expression_stmt'. For input "42"
    # (a plain number), 'assignment' will fail at the first token because it
    # expects NAME. So we should see a fail trace line.
    _ast, lines = parse_with_trace("42")
    has_fail = lines.any? { |l| l.end_with?("fail") }
    assert has_fail, "Expected at least one trace line ending with 'fail'"
  end

  # -------------------------------------------------------------------------
  # Format: type and value appear inside parentheses
  # -------------------------------------------------------------------------

  def test_trace_line_contains_token_type
    _ast, lines = parse_with_trace("42")
    # At least one line should mention a token type. The NUMBER token's type
    # string is "NUMBER" (from TokenType::NUMBER.to_s).
    has_number = lines.any? { |l| l.include?("NUMBER") }
    assert has_number, "Expected at least one trace line mentioning NUMBER token type"
  end

  def test_trace_line_format_matches_spec
    # Verify a specific line matches the documented format:
    #   [TRACE] rule '<name>' at token <N> (<TYPE> "<value>") → match|fail
    _ast, lines = parse_with_trace("42")
    pattern = /\A\[TRACE\] rule '[a-z_]+' at token \d+ \([A-Z_]+ ".*"\) \u2192 (match|fail)\z/
    matching = lines.select { |l| l.match?(pattern) }
    assert matching.length > 0,
      "No trace lines matched the expected format. Got:\n#{lines.first(5).join("\n")}"
  end

  # -------------------------------------------------------------------------
  # Assignment input also produces trace
  # -------------------------------------------------------------------------

  def test_trace_for_assignment_input
    _ast, lines = parse_with_trace("x = 42\n")
    assert lines.length > 0
    # assignment rule should appear as a match
    has_assignment_match = lines.any? { |l| l.include?("'assignment'") && l.end_with?("match") }
    assert has_assignment_match, "Expected assignment rule to match for 'x = 42'"
  end
end
