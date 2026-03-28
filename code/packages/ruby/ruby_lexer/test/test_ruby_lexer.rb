# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_ruby_lexer"

# ================================================================
# Tests for the Ruby Lexer
# ================================================================
#
# Meta-circularity: Ruby code tokenizing Ruby code.
# The grammar-driven approach means the Ruby lexer works the same
# way as all other lexers — we just swap in ruby.tokens.
# ================================================================

TT_RUBY = CodingAdventures::Lexer::TokenType

class TestRubyLexer < Minitest::Test
  def tokenize(source) = CodingAdventures::RubyLexer.tokenize(source)
  def first_token(source) = tokenize(source).first
  def non_eof(source) = tokenize(source).reject { |t| t.type == TT_RUBY::EOF }

  def test_version_exists
    refute_nil CodingAdventures::RubyLexer::VERSION
  end

  def test_simple_name
    tok = first_token("x")
    assert_equal TT_RUBY::NAME, tok.type
    assert_equal "x", tok.value
  end

  def test_integer
    # ruby.tokens uses NUMBER (not INTEGER) for numeric literals
    tok = first_token("42")
    assert_equal TT_RUBY::NUMBER, tok.type
    assert_equal "42", tok.value
  end

  def test_string_double_quoted
    tok = first_token('"hello"')
    assert_equal TT_RUBY::STRING, tok.type
    # GrammarLexer strips surrounding quote characters from string values
    assert_equal "hello", tok.value
  end

  def test_string_single_quoted_not_supported
    # ruby.tokens only defines STRING for double-quoted strings.
    # Single-quoted strings raise a LexerError since the grammar doesn't cover them.
    assert_raises(CodingAdventures::Lexer::LexerError) do
      tokenize("'hello'")
    end
  end

  def test_assignment
    tokens = non_eof("x = 42")
    values = tokens.map(&:value).reject { |v| v =~ /\A\s+\z/ }
    assert_includes values, "x"
    assert_includes values, "="
    assert_includes values, "42"
  end

  def test_keyword_def
    tok = first_token("def")
    assert_equal TT_RUBY::KEYWORD, tok.type
    assert_equal "def", tok.value
  end

  def test_keyword_end
    tok = first_token("end")
    assert_equal TT_RUBY::KEYWORD, tok.type
    assert_equal "end", tok.value
  end

  def test_keyword_if
    tok = first_token("if")
    assert_equal TT_RUBY::KEYWORD, tok.type
    assert_equal "if", tok.value
  end

  def test_keyword_nil
    tok = first_token("nil")
    assert_equal TT_RUBY::KEYWORD, tok.type
    assert_equal "nil", tok.value
  end

  def test_keyword_true
    tok = first_token("true")
    assert_equal TT_RUBY::KEYWORD, tok.type
    assert_equal "true", tok.value
  end

  def test_keyword_false
    tok = first_token("false")
    assert_equal TT_RUBY::KEYWORD, tok.type
    assert_equal "false", tok.value
  end

  def test_plus = assert_equal TT_RUBY::PLUS, first_token("+").type
  def test_minus = assert_equal TT_RUBY::MINUS, first_token("-").type
  def test_star = assert_equal TT_RUBY::STAR, first_token("*").type
  def test_slash = assert_equal TT_RUBY::SLASH, first_token("/").type
  def test_equals = assert_equal TT_RUBY::EQUALS, first_token("=").type
  def test_lparen = assert_equal TT_RUBY::LPAREN, first_token("(").type
  def test_rparen = assert_equal TT_RUBY::RPAREN, first_token(")").type

  def test_method_def
    tokens = non_eof("def greet(name)\n  puts name\nend")
    kws = tokens.select { |t| t.type == TT_RUBY::KEYWORD }.map(&:value)
    assert_includes kws, "def"
    assert_includes kws, "end"
  end

  def test_ends_with_eof
    assert_equal TT_RUBY::EOF, tokenize("x = 1").last.type
  end

  def test_empty_gives_eof
    tokens = tokenize("")
    assert_equal 1, tokens.size
    assert_equal TT_RUBY::EOF, tokens.first.type
  end
end
