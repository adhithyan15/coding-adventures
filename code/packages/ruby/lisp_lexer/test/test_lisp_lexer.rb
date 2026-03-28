# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_lisp_lexer"

# ================================================================
# Tests for the Lisp Lexer
# ================================================================
#
# The Lisp lexer must handle number-vs-symbol ambiguity:
#   - "-42" should be NUMBER, not SYMBOL(-) + NUMBER(42)
#   - "+" should be SYMBOL, not an error
#   - "define" should be SYMBOL (no keyword distinction in Lisp)
#
# Comments (;) and whitespace are skipped automatically.
# ================================================================

TT_LISP = CodingAdventures::Lexer::TokenType

class TestLispLexer < Minitest::Test
  def tokenize(source) = CodingAdventures::LispLexer.tokenize(source)
  def first_token(source) = tokenize(source).first
  def types(source) = tokenize(source).map(&:type)
  def values(source) = tokenize(source).map(&:value)

  def test_version_exists
    refute_nil CodingAdventures::LispLexer::VERSION
  end

  def test_lparen
    assert_equal TT_LISP::LPAREN, first_token("(").type
  end

  def test_rparen
    assert_equal TT_LISP::RPAREN, first_token(")").type
  end

  def test_number_integer
    tok = first_token("42")
    assert_equal "NUMBER", tok.type.to_s
    assert_equal "42", tok.value
  end

  def test_number_negative
    tok = first_token("-42")
    assert_equal "NUMBER", tok.type.to_s
    assert_equal "-42", tok.value
  end

  def test_symbol_define
    tok = first_token("define")
    assert_equal "SYMBOL", tok.type.to_s
    assert_equal "define", tok.value
  end

  def test_symbol_plus
    tok = first_token("+")
    assert_equal "SYMBOL", tok.type.to_s
    assert_equal "+", tok.value
  end

  def test_symbol_minus
    tok = first_token("-")
    assert_equal "SYMBOL", tok.type.to_s
    assert_equal "-", tok.value
  end

  def test_symbol_star
    tok = first_token("*")
    assert_equal "SYMBOL", tok.type.to_s
    assert_equal "*", tok.value
  end

  def test_symbol_predicate
    tok = first_token("nil?")
    assert_equal "SYMBOL", tok.type.to_s
    assert_equal "nil?", tok.value
  end

  def test_string
    tok = first_token('"hello world"')
    assert_equal "STRING", tok.type.to_s
    # GrammarLexer strips surrounding quote characters
    assert_equal "hello world", tok.value
  end

  def test_quote_apostrophe
    tok = first_token("'")
    assert_equal "QUOTE", tok.type.to_s
  end

  def test_dot
    tok = first_token(".")
    assert_equal "DOT", tok.type.to_s
  end

  def test_simple_expression
    # (+ 1 2) → LPAREN SYMBOL(+) NUMBER(1) NUMBER(2) RPAREN EOF
    ts = tokenize("(+ 1 2)")
    non_eof = ts.reject { |t| t.type == TT_LISP::EOF }
    assert_equal 5, non_eof.size
    assert_equal TT_LISP::LPAREN, non_eof[0].type
    assert_equal "SYMBOL", non_eof[1].type.to_s
    assert_equal "+", non_eof[1].value
    assert_equal "NUMBER", non_eof[2].type.to_s
    assert_equal "1", non_eof[2].value
    assert_equal "NUMBER", non_eof[3].type.to_s
    assert_equal "2", non_eof[3].value
    assert_equal TT_LISP::RPAREN, non_eof[4].type
  end

  def test_define_expression
    ts = tokenize("(define x 42)")
    syms = ts.select { |t| t.type.to_s == "SYMBOL" }.map(&:value)
    assert_includes syms, "define"
    assert_includes syms, "x"
  end

  def test_comment_skipped
    ts = tokenize("; this is a comment\n(+ 1 2)")
    non_eof = ts.reject { |t| t.type == TT_LISP::EOF }
    # Comment should be skipped; tokens start with LPAREN
    assert_equal TT_LISP::LPAREN, non_eof.first.type
  end

  def test_ends_with_eof
    assert_equal TT_LISP::EOF, tokenize("(+ 1 2)").last.type
  end

  def test_empty_gives_eof
    tokens = tokenize("")
    assert_equal 1, tokens.size
    assert_equal TT_LISP::EOF, tokens.first.type
  end

  def test_quoted_symbol
    ts = tokenize("'foo")
    assert_equal "QUOTE", ts[0].type.to_s
    assert_equal "SYMBOL", ts[1].type.to_s
    assert_equal "foo", ts[1].value
  end

  def test_dotted_pair
    ts = tokenize("(a . b)")
    dot = ts.find { |t| t.type.to_s == "DOT" }
    refute_nil dot
  end
end
