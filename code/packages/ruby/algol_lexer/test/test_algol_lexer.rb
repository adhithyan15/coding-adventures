# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the ALGOL 60 Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with algol.tokens, correctly tokenizes ALGOL 60 source text.
#
# ALGOL 60 is more complex to lex than JSON because it has:
#   - Keywords (reclassified from IDENT after a full-token match)
#   - Identifier disambiguation (begin vs beginning)
#   - Multi-character operators that share prefixes (:= vs :, ** vs *)
#   - Comment syntax: comment <text up to ;>
#   - Two numeric types: INTEGER_LIT and REAL_LIT (with exponent forms)
#   - String literals with single-quote delimiters
#
# We are not testing the lexer engine itself (that is tested in the
# lexer gem) -- we are testing that the ALGOL 60 token grammar file
# correctly describes ALGOL 60's lexical rules.
# ================================================================

class TestAlgolLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  # ---------------------------------------------------------------
  # ALGOL-specific token type strings (not in TokenType::ALL).
  # The GrammarLexer uses raw string names for grammar-defined tokens.
  # ---------------------------------------------------------------
  INTEGER_LIT_TYPE = "INTEGER_LIT"
  REAL_LIT_TYPE    = "REAL_LIT"
  STRING_LIT_TYPE  = "STRING_LIT"
  IDENT_TYPE       = "NAME"
  ASSIGN_TYPE      = "ASSIGN"
  POWER_TYPE       = "POWER"
  LEQ_TYPE         = "LEQ"
  GEQ_TYPE         = "GEQ"
  NEQ_TYPE         = "NEQ"
  PLUS_TYPE        = "PLUS"
  MINUS_TYPE       = "MINUS"
  STAR_TYPE        = "STAR"
  SLASH_TYPE       = "SLASH"
  CARET_TYPE       = "CARET"
  EQ_TYPE          = "EQ"
  LT_TYPE          = "LT"
  GT_TYPE          = "GT"
  LPAREN_TYPE      = "LPAREN"
  RPAREN_TYPE      = "RPAREN"
  LBRACKET_TYPE    = "LBRACKET"
  RBRACKET_TYPE    = "RBRACKET"
  SEMICOLON_TYPE   = "SEMICOLON"
  COMMA_TYPE       = "COMMA"
  COLON_TYPE       = "COLON"

  # ------------------------------------------------------------------
  # Helper: tokenize source and provide convenient accessors
  # ------------------------------------------------------------------

  def tokenize(source)
    CodingAdventures::AlgolLexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Keywords: all 30 ALGOL 60 keywords
  # ------------------------------------------------------------------
  # ALGOL 60 keywords are reclassified from IDENT. The grammar marks
  # them as case-insensitive: BEGIN, Begin, and begin all produce a
  # BEGIN keyword token.
  #
  # Keyword list (from algol.tokens):
  #   Block:       begin, end
  #   Control:     if, then, else, for, do, step, until, while, goto
  #   Declaration: switch, procedure, own, array, label, value
  #   Types:       integer, real, boolean, string
  #   Boolean:     true, false, not, and, or, impl, eqv
  #   Arithmetic:  div, mod
  #   Comment:     comment (consumed with following text up to ;)

  def test_keyword_begin
    tokens = tokenize("begin")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal "begin", non_eof[0].value
  end

  def test_keyword_end
    tokens = tokenize("end")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "end", non_eof[0].value
  end

  def test_keyword_if
    tokens = tokenize("if")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "if", non_eof[0].value
  end

  def test_keyword_then
    tokens = tokenize("then")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "then", non_eof[0].value
  end

  def test_keyword_else
    tokens = tokenize("else")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "else", non_eof[0].value
  end

  def test_keyword_for
    tokens = tokenize("for")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "for", non_eof[0].value
  end

  def test_keyword_do
    tokens = tokenize("do")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "do", non_eof[0].value
  end

  def test_keyword_step
    tokens = tokenize("step")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "step", non_eof[0].value
  end

  def test_keyword_until
    tokens = tokenize("until")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "until", non_eof[0].value
  end

  def test_keyword_while
    tokens = tokenize("while")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "while", non_eof[0].value
  end

  def test_keyword_goto
    tokens = tokenize("goto")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "goto", non_eof[0].value
  end

  def test_keyword_switch
    tokens = tokenize("switch")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "switch", non_eof[0].value
  end

  def test_keyword_procedure
    tokens = tokenize("procedure")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "procedure", non_eof[0].value
  end

  def test_keyword_array
    tokens = tokenize("array")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "array", non_eof[0].value
  end

  def test_keyword_value
    tokens = tokenize("value")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "value", non_eof[0].value
  end

  def test_keyword_integer
    tokens = tokenize("integer")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "integer", non_eof[0].value
  end

  def test_keyword_real
    tokens = tokenize("real")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "real", non_eof[0].value
  end

  def test_keyword_boolean
    tokens = tokenize("boolean")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "boolean", non_eof[0].value
  end

  def test_keyword_string
    tokens = tokenize("string")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "string", non_eof[0].value
  end

  def test_keyword_true
    tokens = tokenize("true")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "true", non_eof[0].value
  end

  def test_keyword_false
    tokens = tokenize("false")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "false", non_eof[0].value
  end

  def test_keyword_not
    tokens = tokenize("not")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "not", non_eof[0].value
  end

  def test_keyword_and
    tokens = tokenize("and")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "and", non_eof[0].value
  end

  def test_keyword_or
    tokens = tokenize("or")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "or", non_eof[0].value
  end

  def test_keyword_impl
    tokens = tokenize("impl")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "impl", non_eof[0].value
  end

  def test_keyword_eqv
    tokens = tokenize("eqv")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "eqv", non_eof[0].value
  end

  def test_keyword_div
    tokens = tokenize("div")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "div", non_eof[0].value
  end

  def test_keyword_mod
    tokens = tokenize("mod")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal "mod", non_eof[0].value
  end

  # ------------------------------------------------------------------
  # Identifier disambiguation
  # ------------------------------------------------------------------
  # Keywords only match when the entire token is a keyword. A longer
  # identifier that starts with a keyword is still an IDENT.
  # "beginning" must not become BEGIN + IDENT("ning").

  def test_keyword_boundary_begin
    # "begin" alone is a keyword, "beginning" is an IDENT
    tokens = tokenize("beginning")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal "beginning", non_eof[0].value
  end

  def test_keyword_boundary_integer
    tokens = tokenize("integers")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal "integers", non_eof[0].value
  end

  def test_keyword_boundary_for
    tokens = tokenize("format")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal "format", non_eof[0].value
  end

  # ------------------------------------------------------------------
  # Identifiers
  # ------------------------------------------------------------------
  # ALGOL identifiers: letter followed by zero or more letters or digits.
  # No underscores allowed in original ALGOL 60.

  def test_simple_ident
    tokens = tokenize("x")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal "x", non_eof[0].value
  end

  def test_multi_char_ident
    tokens = tokenize("sum")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal "sum", non_eof[0].value
  end

  def test_camel_case_ident
    tokens = tokenize("customerName")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal "customerName", non_eof[0].value
  end

  def test_ident_with_digit
    tokens = tokenize("A1")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal "A1", non_eof[0].value
  end

  # ------------------------------------------------------------------
  # Integer literals
  # ------------------------------------------------------------------

  def test_integer_zero
    tokens = tokenize("0")
    assert_equal INTEGER_LIT_TYPE, tokens[0].type
    assert_equal "0", tokens[0].value
  end

  def test_integer_42
    tokens = tokenize("42")
    assert_equal INTEGER_LIT_TYPE, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_integer_1000
    tokens = tokenize("1000")
    assert_equal INTEGER_LIT_TYPE, tokens[0].type
    assert_equal "1000", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Real literals
  # ------------------------------------------------------------------
  # ALGOL real literals have the form:
  #   3.14        integer + fractional part
  #   1.5E3       integer + exponent (1500.0)
  #   1.5E-3      integer + negative exponent (0.0015)
  #   100E2       integer + exponent, no decimal point (10000.0)
  #
  # REAL_LIT must come before INTEGER_LIT in the grammar so "3.14"
  # matches REAL_LIT not INTEGER_LIT.

  def test_real_decimal
    tokens = tokenize("3.14")
    assert_equal REAL_LIT_TYPE, tokens[0].type
    assert_equal "3.14", tokens[0].value
  end

  def test_real_with_exponent
    tokens = tokenize("1.5E3")
    assert_equal REAL_LIT_TYPE, tokens[0].type
    assert_equal "1.5E3", tokens[0].value
  end

  def test_real_with_negative_exponent
    tokens = tokenize("1.5E-3")
    assert_equal REAL_LIT_TYPE, tokens[0].type
    assert_equal "1.5E-3", tokens[0].value
  end

  def test_real_integer_with_exponent
    # 100E2 = 10000.0 — no decimal point, but the exponent makes it a real
    tokens = tokenize("100E2")
    assert_equal REAL_LIT_TYPE, tokens[0].type
    assert_equal "100E2", tokens[0].value
  end

  def test_real_vs_integer_disambiguation
    # "3.14" should be ONE REAL_LIT token, not INTEGER_LIT + something
    tokens = tokenize("3.14")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal REAL_LIT_TYPE, non_eof[0].type
  end

  # ------------------------------------------------------------------
  # String literals
  # ------------------------------------------------------------------
  # ALGOL 60 strings are single-quoted. Unlike C or Ruby, there are
  # no escape sequences — a single quote cannot appear inside a string.

  def test_string_hello
    tokens = tokenize("'hello'")
    assert_equal STRING_LIT_TYPE, tokens[0].type
    # The lexer strips quotes; value contains just the content
    assert_equal "hello", tokens[0].value
  end

  def test_string_with_spaces
    tokens = tokenize("'x = 5'")
    assert_equal STRING_LIT_TYPE, tokens[0].type
    assert_equal "x = 5", tokens[0].value
  end

  def test_empty_string
    tokens = tokenize("''")
    assert_equal STRING_LIT_TYPE, tokens[0].type
    assert_equal "", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Operators: multi-character (must come before single-char)
  # ------------------------------------------------------------------

  def test_assign_operator
    tokens = tokenize(":=")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal ASSIGN_TYPE, non_eof[0].type
    assert_equal ":=", non_eof[0].value
  end

  def test_power_operator
    tokens = tokenize("**")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal POWER_TYPE, non_eof[0].type
    assert_equal "**", non_eof[0].value
  end

  def test_leq_operator
    tokens = tokenize("<=")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal LEQ_TYPE, non_eof[0].type
    assert_equal "<=", non_eof[0].value
  end

  def test_geq_operator
    tokens = tokenize(">=")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal GEQ_TYPE, non_eof[0].type
    assert_equal ">=", non_eof[0].value
  end

  def test_neq_operator
    tokens = tokenize("!=")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal NEQ_TYPE, non_eof[0].type
    assert_equal "!=", non_eof[0].value
  end

  # ------------------------------------------------------------------
  # Operators: single-character
  # ------------------------------------------------------------------

  def test_plus_operator
    tokens = tokenize("+")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal PLUS_TYPE, non_eof[0].type
    assert_equal "+", non_eof[0].value
  end

  def test_minus_operator
    tokens = tokenize("-")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal MINUS_TYPE, non_eof[0].type
    assert_equal "-", non_eof[0].value
  end

  def test_star_operator
    tokens = tokenize("*")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal STAR_TYPE, non_eof[0].type
    assert_equal "*", non_eof[0].value
  end

  def test_slash_operator
    tokens = tokenize("/")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal SLASH_TYPE, non_eof[0].type
    assert_equal "/", non_eof[0].value
  end

  def test_caret_operator
    tokens = tokenize("^")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal CARET_TYPE, non_eof[0].type
    assert_equal "^", non_eof[0].value
  end

  def test_eq_operator
    tokens = tokenize("=")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal EQ_TYPE, non_eof[0].type
    assert_equal "=", non_eof[0].value
  end

  def test_lt_operator
    tokens = tokenize("<")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal LT_TYPE, non_eof[0].type
    assert_equal "<", non_eof[0].value
  end

  def test_gt_operator
    tokens = tokenize(">")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal GT_TYPE, non_eof[0].type
    assert_equal ">", non_eof[0].value
  end

  # ------------------------------------------------------------------
  # Delimiters
  # ------------------------------------------------------------------

  def test_lparen
    tokens = tokenize("(")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal LPAREN_TYPE, non_eof[0].type
    assert_equal "(", non_eof[0].value
  end

  def test_rparen
    tokens = tokenize(")")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal RPAREN_TYPE, non_eof[0].type
    assert_equal ")", non_eof[0].value
  end

  def test_lbracket
    tokens = tokenize("[")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal LBRACKET_TYPE, non_eof[0].type
    assert_equal "[", non_eof[0].value
  end

  def test_rbracket
    tokens = tokenize("]")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal RBRACKET_TYPE, non_eof[0].type
    assert_equal "]", non_eof[0].value
  end

  def test_semicolon
    tokens = tokenize(";")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal SEMICOLON_TYPE, non_eof[0].type
    assert_equal ";", non_eof[0].value
  end

  def test_comma
    tokens = tokenize(",")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal COMMA_TYPE, non_eof[0].type
    assert_equal ",", non_eof[0].value
  end

  def test_colon
    tokens = tokenize(":")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal COLON_TYPE, non_eof[0].type
    assert_equal ":", non_eof[0].value
  end

  # ------------------------------------------------------------------
  # Disambiguation: multi-char operators vs single-char prefixes
  # ------------------------------------------------------------------
  # These tests are critical for correctness. A naive lexer could
  # misparse ":=" as COLON + EQ. The grammar must try longer patterns
  # before shorter ones (first-match-wins priority ordering).

  def test_assign_not_colon_plus_eq
    # ":=" must be ONE ASSIGN token, not COLON + EQ
    tokens = tokenize(":=")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal ASSIGN_TYPE, non_eof[0].type
  end

  def test_power_not_star_plus_star
    # "**" must be ONE POWER token, not STAR + STAR
    tokens = tokenize("**")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal POWER_TYPE, non_eof[0].type
  end

  def test_leq_not_lt_plus_eq
    # "<=" must be ONE LEQ token, not LT + EQ
    tokens = tokenize("<=")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal LEQ_TYPE, non_eof[0].type
  end

  def test_geq_not_gt_plus_eq
    # ">=" must be ONE GEQ token, not GT + EQ
    tokens = tokenize(">=")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal GEQ_TYPE, non_eof[0].type
  end

  def test_neq_not_something_plus_eq
    # "!=" must be ONE NEQ token
    tokens = tokenize("!=")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal NEQ_TYPE, non_eof[0].type
  end

  def test_colon_alone_after_assign
    # After ":=" is parsed, a standalone ":" is still COLON
    tokens = tokenize(":= :")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal ASSIGN_TYPE, non_eof[0].type
    assert_equal COLON_TYPE, non_eof[1].type
  end

  def test_star_alone_after_power
    # After "**" is parsed, a standalone "*" is still STAR
    tokens = tokenize("** *")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal POWER_TYPE, non_eof[0].type
    assert_equal STAR_TYPE, non_eof[1].type
  end

  # ------------------------------------------------------------------
  # Comment skipping
  # ------------------------------------------------------------------
  # ALGOL 60 comment syntax: the word "comment" followed by any text
  # up to and including the next semicolon. The entire comment
  # (including "comment" and the terminating ";") is consumed silently.

  def test_comment_skipped
    # comment this is ignored; x := 1
    # Only x := 1 should produce tokens
    tokens = tokenize("comment this is ignored; x := 1")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    # Expect: IDENT(x), ASSIGN, INTEGER_LIT(1)
    assert_equal 3, non_eof.length
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal "x", non_eof[0].value
    assert_equal ASSIGN_TYPE, non_eof[1].type
    assert_equal INTEGER_LIT_TYPE, non_eof[2].type
    assert_equal "1", non_eof[2].value
  end

  def test_comment_with_multiple_words
    tokens = tokenize("comment this has many words and punctuation!; y := 2")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 3, non_eof.length
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal "y", non_eof[0].value
  end

  # ------------------------------------------------------------------
  # Whitespace handling
  # ------------------------------------------------------------------
  # ALGOL 60 is free-format. Whitespace is completely insignificant.
  # No NEWLINE, INDENT, or DEDENT tokens are emitted.

  def test_whitespace_skipped
    tokens = tokenize("  42  ")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof.length
    assert_equal INTEGER_LIT_TYPE, non_eof[0].type
    assert_equal "42", non_eof[0].value
  end

  def test_no_spaces_same_as_with_spaces
    # "x:=1" and "x := 1" should produce identical token sequences
    tokens_compact  = tokenize("x:=1").reject { |t| t.type == TT::EOF }
    tokens_spaced   = tokenize("x := 1").reject { |t| t.type == TT::EOF }
    assert_equal tokens_compact.map(&:type),  tokens_spaced.map(&:type)
    assert_equal tokens_compact.map(&:value), tokens_spaced.map(&:value)
  end

  def test_newlines_are_whitespace
    tokens = tokenize("x\n:=\n1")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 3, non_eof.length
    types = non_eof.map(&:type)
    assert_equal [IDENT_TYPE, ASSIGN_TYPE, INTEGER_LIT_TYPE], types
  end

  def test_tabs_are_whitespace
    tokens = tokenize("x\t:=\t1")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 3, non_eof.length
  end

  # ------------------------------------------------------------------
  # Multi-token sequences
  # ------------------------------------------------------------------

  def test_arithmetic_expression
    # x := 1 + 2 * 3
    tokens = tokenize("x := 1 + 2 * 3")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    types = non_eof.map(&:type)
    expected = [IDENT_TYPE, ASSIGN_TYPE, INTEGER_LIT_TYPE, PLUS_TYPE,
                INTEGER_LIT_TYPE, STAR_TYPE, INTEGER_LIT_TYPE]
    assert_equal expected, types
    values = non_eof.map(&:value)
    assert_equal ["x", ":=", "1", "+", "2", "*", "3"], values
  end

  def test_power_expression
    # x ** 2
    tokens = tokenize("x ** 2")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal IDENT_TYPE, non_eof[0].type
    assert_equal POWER_TYPE, non_eof[1].type
    assert_equal INTEGER_LIT_TYPE, non_eof[2].type
  end

  def test_comparison_expression
    # a <= b
    tokens = tokenize("a <= b")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_equal LEQ_TYPE, non_eof[1].type
  end

  def test_begin_end_block
    # begin integer x; x := 42 end
    source = "begin integer x; x := 42 end"
    tokens = tokenize(source)
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    values = non_eof.map(&:value)
    assert_equal "begin",   values[0]
    assert_equal "integer", values[1]
    assert_equal "x",       values[2]
    assert_equal ";",       values[3]
    assert_equal "x",       values[4]
    assert_equal ":=",      values[5]
    assert_equal "42",      values[6]
    assert_equal "end",     values[7]
  end

  def test_if_then_else
    # if x > 0 then y := 1 else y := 0
    source = "if x > 0 then y := 1 else y := 0"
    tokens = tokenize(source)
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    values = non_eof.map(&:value)
    assert_equal "if",   values[0]
    assert_equal "then", values[4]
    assert_equal "else", values[8]
  end

  def test_for_loop
    # for i := 1 step 1 until 10 do x := x + i
    source = "for i := 1 step 1 until 10 do x := x + i"
    tokens = tokenize(source)
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    values = non_eof.map(&:value)
    assert_equal "for",   values[0]
    assert_equal "step",  values[4]
    assert_equal "until", values[6]
    assert_equal "do",    values[8]
  end

  def test_boolean_operators
    # a and b or not c
    source = "a and b or not c"
    tokens = tokenize(source)
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    values = non_eof.map(&:value)
    assert_equal "and", values[1]
    assert_equal "or",  values[3]
    assert_equal "not", values[4]
  end

  def test_div_mod_operators
    # n div 2 mod 3
    source = "n div 2 mod 3"
    tokens = tokenize(source)
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    values = non_eof.map(&:value)
    assert_equal "div", values[1]
    assert_equal "mod", values[3]
  end

  def test_procedure_declaration
    # procedure p(x); integer x; begin end
    source = "procedure p(x); integer x; begin end"
    tokens = tokenize(source)
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    values = non_eof.map(&:value)
    assert_equal "procedure", values[0]
    assert_equal "integer",   values[4]
  end

  def test_array_declaration
    # integer array A[1:10]
    source = "integer array A[1:10]"
    tokens = tokenize(source)
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    values = non_eof.map(&:value)
    assert_equal "integer", values[0]
    assert_equal "array",   values[1]
    assert_equal "A",       values[2]
    assert_equal "[",       values[3]
    assert_equal "1",       values[4]
    assert_equal ":",       values[5]
    assert_equal "10",      values[6]
    assert_equal "]",       values[7]
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::AlgolLexer::ALGOL_TOKENS_PATH),
      "algol.tokens file should exist at #{CodingAdventures::AlgolLexer::ALGOL_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Line tracking
  # ------------------------------------------------------------------

  def test_line_tracking
    source = "begin\n  integer x;\n  x := 42\nend"
    tokens = tokenize(source)
    # "x" on line 2 (after the newline after "begin")
    ident_tokens = tokens.select { |t| t.type == IDENT_TYPE && t.value == "x" }
    refute_empty ident_tokens, "Expected IDENT 'x' tokens"
    assert_equal 2, ident_tokens.first.line, "Expected first 'x' on line 2"
  end

  # ------------------------------------------------------------------
  # EOF token
  # ------------------------------------------------------------------
  # Every token stream must end with an EOF token.

  def test_eof_token
    tokens = tokenize("42")
    assert_equal TT::EOF, tokens.last.type
  end

  def test_empty_input_eof
    tokens = tokenize("")
    assert_equal TT::EOF, tokens.last.type
  end
end
