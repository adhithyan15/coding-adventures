# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the ECMAScript 1 (ES1) Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with es1.tokens, correctly tokenizes ES1 JavaScript source code.
#
# ES1 is the first standardized version of JavaScript (1997). These
# tests focus on features specific to ES1:
#   - var declarations (no let/const)
#   - == and != (no strict equality === or !==)
#   - Basic keywords, operators, and delimiters
#   - $ in identifiers
#   - Hex numbers, floats, scientific notation
#   - Single and double quoted strings
#   - Compound assignment operators (+=, -=, etc.)
#   - Bitwise and shift operators
#   - Increment/decrement operators
# ================================================================

class TestEcmascriptEs1Lexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  def tokenize(source)
    CodingAdventures::EcmascriptEs1Lexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Basic expression: var x = 1 + 2;
  # ------------------------------------------------------------------

  def test_var_assignment
    tokens = tokenize("var x = 1 + 2;")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::NAME, TT::EQUALS, TT::NUMBER,
      TT::PLUS, TT::NUMBER, TT::SEMICOLON, TT::EOF], types
  end

  def test_var_assignment_values
    tokens = tokenize("var x = 1 + 2;")
    values = tokens.map(&:value)
    assert_equal ["var", "x", "=", "1", "+", "2", ";", ""], values
  end

  # ------------------------------------------------------------------
  # ES1 keywords
  # ------------------------------------------------------------------

  def test_keyword_var
    tokens = tokenize("var")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "var", tokens[0].value
  end

  def test_keyword_function
    tokens = tokenize("function")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "function", tokens[0].value
  end

  def test_keyword_if
    tokens = tokenize("if")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "if", tokens[0].value
  end

  def test_keyword_while
    tokens = tokenize("while")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "while", tokens[0].value
  end

  def test_keyword_for
    tokens = tokenize("for")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "for", tokens[0].value
  end

  def test_keyword_return
    tokens = tokenize("return")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "return", tokens[0].value
  end

  def test_keyword_this
    tokens = tokenize("this")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "this", tokens[0].value
  end

  def test_keyword_new
    tokens = tokenize("new")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "new", tokens[0].value
  end

  def test_keyword_delete
    tokens = tokenize("delete")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "delete", tokens[0].value
  end

  def test_keyword_typeof
    tokens = tokenize("typeof")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "typeof", tokens[0].value
  end

  def test_keyword_void
    tokens = tokenize("void")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "void", tokens[0].value
  end

  def test_keyword_switch
    tokens = tokenize("switch")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "switch", tokens[0].value
  end

  def test_keyword_case
    tokens = tokenize("case")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "case", tokens[0].value
  end

  def test_keyword_break
    tokens = tokenize("break")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "break", tokens[0].value
  end

  def test_keyword_continue
    tokens = tokenize("continue")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "continue", tokens[0].value
  end

  def test_keyword_with
    tokens = tokenize("with")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "with", tokens[0].value
  end

  def test_boolean_and_null_keywords
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
  # ES1 does NOT have strict equality -- verify == and != only
  # ------------------------------------------------------------------

  def test_equality_operators
    tokens = tokenize("x == 1")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS_EQUALS, TT::NUMBER, TT::EOF], types
  end

  def test_not_equals
    tokens = tokenize("x != 1")
    assert_equal "!=", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Comparison operators
  # ------------------------------------------------------------------

  def test_less_equals
    tokens = tokenize("a <= b")
    assert_equal "<=", tokens[1].value
  end

  def test_greater_equals
    tokens = tokenize("a >= b")
    assert_equal ">=", tokens[1].value
  end

  def test_less_than
    tokens = tokenize("a < b")
    assert_equal "<", tokens[1].value
  end

  def test_greater_than
    tokens = tokenize("a > b")
    assert_equal ">", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Compound assignment operators
  # ------------------------------------------------------------------

  def test_plus_equals
    tokens = tokenize("x += 1")
    assert_equal "+=", tokens[1].value
  end

  def test_minus_equals
    tokens = tokenize("x -= 1")
    assert_equal "-=", tokens[1].value
  end

  def test_star_equals
    tokens = tokenize("x *= 2")
    assert_equal "*=", tokens[1].value
  end

  def test_slash_equals
    tokens = tokenize("x /= 2")
    assert_equal "/=", tokens[1].value
  end

  def test_percent_equals
    tokens = tokenize("x %= 3")
    assert_equal "%=", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Bitwise and shift operators
  # ------------------------------------------------------------------

  def test_ampersand
    tokens = tokenize("a & b")
    assert_equal "&", tokens[1].value
  end

  def test_pipe
    tokens = tokenize("a | b")
    assert_equal "|", tokens[1].value
  end

  def test_caret
    tokens = tokenize("a ^ b")
    assert_equal "^", tokens[1].value
  end

  def test_tilde
    tokens = tokenize("~a")
    assert_equal "~", tokens[0].value
  end

  def test_left_shift
    tokens = tokenize("a << 2")
    assert_equal "<<", tokens[1].value
  end

  def test_right_shift
    tokens = tokenize("a >> 2")
    assert_equal ">>", tokens[1].value
  end

  def test_unsigned_right_shift
    tokens = tokenize("a >>> 2")
    assert_equal ">>>", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Logical operators
  # ------------------------------------------------------------------

  def test_and_and
    tokens = tokenize("a && b")
    assert_equal "&&", tokens[1].value
  end

  def test_or_or
    tokens = tokenize("a || b")
    assert_equal "||", tokens[1].value
  end

  def test_bang
    tokens = tokenize("!a")
    assert_equal "!", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Increment/decrement operators
  # ------------------------------------------------------------------

  def test_plus_plus
    tokens = tokenize("x++")
    assert_equal "++", tokens[1].value
  end

  def test_minus_minus
    tokens = tokenize("x--")
    assert_equal "--", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Ternary operator
  # ------------------------------------------------------------------

  def test_question_mark
    tokens = tokenize("a ? b : c")
    assert_equal "?", tokens[1].value
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

  def test_parentheses
    tokens = tokenize("( )")
    types = tokens.map(&:type)
    assert_equal [TT::LPAREN, TT::RPAREN, TT::EOF], types
  end

  def test_semicolon
    tokens = tokenize(";")
    assert_equal TT::SEMICOLON, tokens[0].type
  end

  def test_comma
    tokens = tokenize(",")
    assert_equal TT::COMMA, tokens[0].type
  end

  def test_colon
    tokens = tokenize(":")
    assert_equal TT::COLON, tokens[0].type
  end

  def test_dot
    tokens = tokenize(".")
    assert_equal TT::DOT, tokens[0].type
  end

  # ------------------------------------------------------------------
  # Identifiers with $ (valid in ES1)
  # ------------------------------------------------------------------

  def test_dollar_sign_identifier
    tokens = tokenize("$foo")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "$foo", tokens[0].value
  end

  def test_underscore_identifier
    tokens = tokenize("_bar")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "_bar", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Numeric literals
  # ------------------------------------------------------------------

  def test_integer
    tokens = tokenize("42")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_float
    tokens = tokenize("3.14")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "3.14", tokens[0].value
  end

  def test_hex_number
    tokens = tokenize("0xFF")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "0xFF", tokens[0].value
  end

  def test_scientific_notation
    tokens = tokenize("1e10")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "1e10", tokens[0].value
  end

  # ------------------------------------------------------------------
  # String literals
  # ------------------------------------------------------------------

  def test_double_quoted_string
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_single_quoted_string
    tokens = tokenize("'world'")
    assert_equal TT::STRING, tokens[0].type
    assert_equal "world", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Comments are skipped
  # ------------------------------------------------------------------

  def test_line_comment_skipped
    tokens = tokenize("var x; // comment")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_equal ["var", "x", ";"], values
  end

  def test_block_comment_skipped
    tokens = tokenize("var /* skip */ x;")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_equal ["var", "x", ";"], values
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::EcmascriptEs1Lexer::ES1_TOKENS_PATH),
      "es1.tokens file should exist at #{CodingAdventures::EcmascriptEs1Lexer::ES1_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Multi-token expressions
  # ------------------------------------------------------------------

  def test_function_declaration_tokens
    tokens = tokenize("function foo(a, b) { return a + b; }")
    types = tokens.map(&:type)
    assert_includes types, TT::KEYWORD  # function, return
    assert_includes types, TT::NAME     # foo, a, b
    assert_includes types, TT::LPAREN
    assert_includes types, TT::RPAREN
    assert_includes types, TT::LBRACE
    assert_includes types, TT::RBRACE
  end

  def test_for_loop_tokens
    tokens = tokenize("for (var i = 0; i < 10; i++) { }")
    values = tokens.map(&:value)
    assert_includes values, "for"
    assert_includes values, "var"
    assert_includes values, "++"
  end
end
