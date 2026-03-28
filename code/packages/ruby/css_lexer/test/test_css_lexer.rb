# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_css_lexer"

TT = CodingAdventures::Lexer::TokenType

class TestCssLexer < Minitest::Test
  def tokenize(source) = CodingAdventures::CssLexer.tokenize(source)
  def first_token(source) = tokenize(source).first

  def test_version_exists
    refute_nil CodingAdventures::CssLexer::VERSION
  end

  def test_basic_ident
    tok = first_token("color")
    assert_equal "IDENT", tok.type.to_s
    assert_equal "color", tok.value
  end

  def test_dimension_px
    tok = first_token("12px")
    assert_equal "DIMENSION", tok.type.to_s
    assert_equal "12px", tok.value
  end

  def test_dimension_em
    tok = first_token("1.5em")
    assert_equal "DIMENSION", tok.type.to_s
    assert_equal "1.5em", tok.value
  end

  def test_dimension_rem
    tok = first_token("2rem")
    assert_equal "DIMENSION", tok.type.to_s
    assert_equal "2rem", tok.value
  end

  def test_dimension_vh
    tok = first_token("100vh")
    assert_equal "DIMENSION", tok.type.to_s
    assert_equal "100vh", tok.value
  end

  def test_percentage
    tok = first_token("50%")
    assert_equal "PERCENTAGE", tok.type.to_s
    assert_equal "50%", tok.value
  end

  def test_percentage_decimal
    tok = first_token("33.3%")
    assert_equal "PERCENTAGE", tok.type.to_s
    assert_equal "33.3%", tok.value
  end

  def test_number_zero
    tok = first_token("0")
    assert_equal "NUMBER", tok.type.to_s
    assert_equal "0", tok.value
  end

  def test_number_float
    tok = first_token("3.14")
    assert_equal "NUMBER", tok.type.to_s
    assert_equal "3.14", tok.value
  end

  def test_function_rgb
    tok = first_token("rgb(")
    assert_equal "FUNCTION", tok.type.to_s
    assert_equal "rgb(", tok.value
  end

  def test_function_rgba
    tok = first_token("rgba(")
    assert_equal "FUNCTION", tok.type.to_s
    assert_equal "rgba(", tok.value
  end

  def test_function_var
    tok = first_token("var(")
    assert_equal "FUNCTION", tok.type.to_s
    assert_equal "var(", tok.value
  end

  def test_hash_short
    tok = first_token("#fff")
    assert_equal "HASH", tok.type.to_s
    assert_equal "#fff", tok.value
  end

  def test_hash_long
    tok = first_token("#336699")
    assert_equal "HASH", tok.type.to_s
    assert_equal "#336699", tok.value
  end

  def test_string_double_quoted
    tok = first_token('"serif"')
    assert_equal "STRING", tok.type.to_s
    # GrammarLexer strips surrounding quote characters from string values
    assert_equal "serif", tok.value
  end

  def test_string_single_quoted
    tok = first_token("'sans-serif'")
    assert_equal "STRING", tok.type.to_s
    # GrammarLexer strips surrounding quote characters from string values
    assert_equal "sans-serif", tok.value
  end

  def test_at_media
    tok = first_token("@media")
    assert_equal "AT_KEYWORD", tok.type.to_s
    assert_equal "@media", tok.value
  end

  def test_custom_property
    tok = first_token("--color")
    assert_equal "CUSTOM_PROPERTY", tok.type.to_s
    assert_equal "--color", tok.value
  end

  def test_custom_property_dashes
    tok = first_token("--font-size-large")
    assert_equal "CUSTOM_PROPERTY", tok.type.to_s
    assert_equal "--font-size-large", tok.value
  end

  def test_colon = assert_equal "COLON", first_token(":").type.to_s
  def test_semicolon = assert_equal "SEMICOLON", first_token(";").type.to_s
  def test_lbrace = assert_equal "LBRACE", first_token("{").type.to_s
  def test_rbrace = assert_equal "RBRACE", first_token("}").type.to_s

  def test_full_rule_tokenizes
    tokens = tokenize("h1 { color: #333; font-size: 2rem; }")
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    refute_empty non_eof
    assert_equal "IDENT", non_eof.first.type.to_s
    assert_equal "h1", non_eof.first.value
    hash_tokens = tokens.select { |t| t.type.to_s == "HASH" }
    assert_equal 1, hash_tokens.size
    assert_equal "#333", hash_tokens.first.value
    dim_tokens = tokens.select { |t| t.type.to_s == "DIMENSION" }
    assert_equal 1, dim_tokens.size
    assert_equal "2rem", dim_tokens.first.value
  end

  def test_ends_with_eof
    assert_equal TT::EOF, tokenize("color: red;").last.type
  end

  def test_empty_string_gives_eof
    tokens = tokenize("")
    assert_equal 1, tokens.size
    assert_equal TT::EOF, tokens.first.type
  end
end
