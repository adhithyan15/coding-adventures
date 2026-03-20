# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# Tests for the Tokenizer DFA and classify_char helper
# ==========================================================================
#
# These tests verify two things:
#
# 1. The classify_char method correctly maps every character to its
#    character class (the DFA's alphabet).
#
# 2. The TOKENIZER_DFA formally matches the tokenizer's actual dispatch
#    behavior. Every character class from "start" transitions to the
#    correct handler state.
#
# 3. The DFA is well-formed: it is complete (every state handles every
#    input), all states are reachable, and the formal model matches
#    what the tokenizer actually does when tokenizing real code.
# ==========================================================================

class TestTokenizerDFA < Minitest::Test
  DFAMod = CodingAdventures::Lexer::TokenizerDFA
  TT = CodingAdventures::Lexer::TokenType
  Tokenizer = CodingAdventures::Lexer::Tokenizer

  # -----------------------------------------------------------------------
  # classify_char tests
  # -----------------------------------------------------------------------
  #
  # These tests ensure that classify_char maps every relevant character
  # to the correct class. The character classes form the DFA's alphabet.
  # Getting these wrong would cause the DFA to dispatch to the wrong
  # sub-routine, producing incorrect tokens.

  def test_classify_char_eof
    assert_equal "eof", DFAMod.classify_char(nil)
  end

  def test_classify_char_digit
    ("0".."9").each do |ch|
      assert_equal "digit", DFAMod.classify_char(ch), "Expected #{ch.inspect} to be 'digit'"
    end
  end

  def test_classify_char_alpha
    %w[a z A Z m M].each do |ch|
      assert_equal "alpha", DFAMod.classify_char(ch), "Expected #{ch.inspect} to be 'alpha'"
    end
  end

  def test_classify_char_underscore
    assert_equal "underscore", DFAMod.classify_char("_")
  end

  def test_classify_char_whitespace
    [" ", "\t", "\r"].each do |ch|
      assert_equal "whitespace", DFAMod.classify_char(ch), "Expected #{ch.inspect} to be 'whitespace'"
    end
  end

  def test_classify_char_newline
    assert_equal "newline", DFAMod.classify_char("\n")
  end

  def test_classify_char_quote
    assert_equal "quote", DFAMod.classify_char('"')
  end

  def test_classify_char_equals
    assert_equal "equals", DFAMod.classify_char("=")
  end

  def test_classify_char_operators
    %w[+ - * /].each do |ch|
      assert_equal "operator", DFAMod.classify_char(ch), "Expected #{ch.inspect} to be 'operator'"
    end
  end

  def test_classify_char_delimiters
    {
      "(" => "open_paren",
      ")" => "close_paren",
      "," => "comma",
      ":" => "colon",
      ";" => "semicolon",
      "{" => "open_brace",
      "}" => "close_brace",
      "[" => "open_bracket",
      "]" => "close_bracket",
      "." => "dot",
      "!" => "bang"
    }.each do |ch, expected|
      assert_equal expected, DFAMod.classify_char(ch), "Expected #{ch.inspect} to be '#{expected}'"
    end
  end

  def test_classify_char_other
    %w[@ # $ % ^ & ~ `].each do |ch|
      assert_equal "other", DFAMod.classify_char(ch), "Expected #{ch.inspect} to be 'other'"
    end
  end

  # -----------------------------------------------------------------------
  # DFA construction tests
  # -----------------------------------------------------------------------
  #
  # These tests verify that the DFA is structurally sound. A well-formed
  # tokenizer DFA should be:
  #
  # - Complete: every (state, input) pair has a defined transition.
  #   This ensures the tokenizer never gets "stuck" -- every character
  #   in every state leads somewhere.
  #
  # - Starting in "start": the initial state is where the tokenizer
  #   begins examining each new character.
  #
  # - All states reachable: no dead states that waste memory.

  def test_dfa_creation
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "start", dfa.current_state
  end

  def test_dfa_is_complete
    dfa = DFAMod.new_tokenizer_dfa
    assert dfa.complete?, "TOKENIZER_DFA should be complete (transition for every state/input pair)"
  end

  def test_dfa_has_no_warnings
    dfa = DFAMod.new_tokenizer_dfa
    warnings = dfa.validate
    assert_empty warnings, "TOKENIZER_DFA should have no validation warnings, got: #{warnings}"
  end

  # -----------------------------------------------------------------------
  # DFA transition tests
  # -----------------------------------------------------------------------
  #
  # These tests verify that each character class from "start" transitions
  # to the correct handler state. This is the formal specification of the
  # tokenizer's dispatch logic.

  def test_dfa_start_to_in_number_on_digit
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "in_number", dfa.process("digit")
  end

  def test_dfa_start_to_in_name_on_alpha
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "in_name", dfa.process("alpha")
  end

  def test_dfa_start_to_in_name_on_underscore
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "in_name", dfa.process("underscore")
  end

  def test_dfa_start_to_in_string_on_quote
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "in_string", dfa.process("quote")
  end

  def test_dfa_start_to_at_newline_on_newline
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "at_newline", dfa.process("newline")
  end

  def test_dfa_start_to_at_whitespace_on_whitespace
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "at_whitespace", dfa.process("whitespace")
  end

  def test_dfa_start_to_in_operator_on_operator
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "in_operator", dfa.process("operator")
  end

  def test_dfa_start_to_in_equals_on_equals
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "in_equals", dfa.process("equals")
  end

  def test_dfa_start_to_in_operator_on_delimiters
    delimiter_classes = %w[
      open_paren close_paren comma colon semicolon
      open_brace close_brace open_bracket close_bracket dot bang
    ]
    delimiter_classes.each do |char_class|
      dfa = DFAMod.new_tokenizer_dfa
      assert_equal "in_operator", dfa.process(char_class),
        "Expected 'start' + '#{char_class}' -> 'in_operator'"
    end
  end

  def test_dfa_start_to_done_on_eof
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "done", dfa.process("eof")
  end

  def test_dfa_start_to_error_on_other
    dfa = DFAMod.new_tokenizer_dfa
    assert_equal "error", dfa.process("other")
  end

  # -----------------------------------------------------------------------
  # Handler states return to "start"
  # -----------------------------------------------------------------------
  #
  # After emitting a token, the lexer returns to the start state. The DFA
  # models this by having all handler states transition to "start" on every
  # symbol.

  def test_dfa_handler_returns_to_start
    handlers = %w[in_number in_name in_string in_operator in_equals at_newline at_whitespace]
    handlers.each do |handler|
      dfa = DFAMod.new_tokenizer_dfa
      # First, get to the handler state via an appropriate symbol
      case handler
      when "in_number" then dfa.process("digit")
      when "in_name" then dfa.process("alpha")
      when "in_string" then dfa.process("quote")
      when "in_operator" then dfa.process("operator")
      when "in_equals" then dfa.process("equals")
      when "at_newline" then dfa.process("newline")
      when "at_whitespace" then dfa.process("whitespace")
      end
      assert_equal handler, dfa.current_state, "Should be in #{handler}"

      # Now process any symbol -- should return to "start"
      result = dfa.process("eof")
      assert_equal "start", result, "#{handler} + any symbol should return to 'start'"
    end
  end

  # -----------------------------------------------------------------------
  # Terminal states loop on themselves
  # -----------------------------------------------------------------------

  def test_dfa_done_loops
    dfa = DFAMod.new_tokenizer_dfa
    dfa.process("eof") # -> done
    assert_equal "done", dfa.current_state

    # Any further input stays in "done"
    result = dfa.process("digit")
    assert_equal "done", result
  end

  def test_dfa_error_loops
    dfa = DFAMod.new_tokenizer_dfa
    dfa.process("other") # -> error
    assert_equal "error", dfa.current_state

    # Any further input stays in "error"
    result = dfa.process("digit")
    assert_equal "error", result
  end

  # -----------------------------------------------------------------------
  # DFA equivalence: formal model matches actual tokenizer behavior
  # -----------------------------------------------------------------------
  #
  # These integration tests prove that the DFA-driven dispatch produces
  # exactly the same tokens as the original implicit dispatch. We tokenize
  # realistic expressions and verify the results match expectations.

  def test_dfa_dispatch_matches_simple_expression
    tokens = Tokenizer.new("x = 42 + y").tokenize
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS, TT::NUMBER, TT::PLUS, TT::NAME, TT::EOF], types
  end

  def test_dfa_dispatch_matches_comparison
    tokens = Tokenizer.new("x == 5").tokenize
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS_EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_dfa_dispatch_matches_string_literal
    tokens = Tokenizer.new('"hello"').tokenize
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_dfa_dispatch_matches_multiline
    tokens = Tokenizer.new("x = 1\ny = 2").tokenize
    types = tokens.map(&:type)
    assert_equal [
      TT::NAME, TT::EQUALS, TT::NUMBER, TT::NEWLINE,
      TT::NAME, TT::EQUALS, TT::NUMBER, TT::EOF
    ], types
  end

  def test_dfa_dispatch_matches_all_delimiters
    tokens = Tokenizer.new("( ) , : ; { } [ ] . !").tokenize
    types = tokens.map(&:type)
    assert_equal [
      TT::LPAREN, TT::RPAREN, TT::COMMA, TT::COLON, TT::SEMICOLON,
      TT::LBRACE, TT::RBRACE, TT::LBRACKET, TT::RBRACKET, TT::DOT,
      TT::BANG, TT::EOF
    ], types
  end

  def test_dfa_dispatch_matches_keywords
    tokens = Tokenizer.new("if x == 1", keywords: ["if"]).tokenize
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "if", tokens[0].value
    assert_equal TT::NAME, tokens[1].type
  end

  def test_dfa_dispatch_matches_realistic_code
    tokens = Tokenizer.new("if x == 1:\n    return \"hello\"", keywords: %w[if return]).tokenize
    # Should tokenize without error -- the DFA-driven dispatch handles
    # all character classes correctly.
    assert tokens.last.type == TT::EOF
    assert tokens.length > 1
  end

  def test_dfa_dispatch_handles_empty_input
    tokens = Tokenizer.new("").tokenize
    assert_equal 1, tokens.length
    assert_equal TT::EOF, tokens[0].type
  end

  def test_dfa_dispatch_handles_only_whitespace
    tokens = Tokenizer.new("   \t  ").tokenize
    assert_equal 1, tokens.length
    assert_equal TT::EOF, tokens[0].type
  end

  def test_dfa_dispatch_handles_unexpected_character
    error = assert_raises(CodingAdventures::Lexer::LexerError) do
      Tokenizer.new("@").tokenize
    end
    assert_includes error.message, "Unexpected character"
  end
end
