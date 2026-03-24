# frozen_string_literal: true

# ================================================================
# Test Suite for CodingAdventures::LatticeLexer
# ================================================================
#
# We test the tokenizer by running source text through
# CodingAdventures::LatticeLexer.tokenize and verifying that the
# resulting token types and values match expectations.
#
# Coverage strategy:
# - All token types defined in lattice.tokens
# - Lattice-specific tokens: VARIABLE, EQUALS_EQUALS, NOT_EQUALS,
#   GREATER_EQUALS, LESS_EQUALS
# - Skip patterns: whitespace, // comments, /* */ comments
# - Error tokens: BAD_STRING, BAD_URL
# - Integration: multi-token Lattice source fragments
# ================================================================

require "minitest/autorun"
require "coding_adventures_lattice_lexer"

# Helper: tokenize source and filter out EOF for cleaner assertions.
def tokenize(source)
  tokens = CodingAdventures::LatticeLexer.tokenize(source)
  tokens.reject { |t| t.type.to_s == "EOF" }
end

# Helper: extract just the type names from a token array.
def types(tokens)
  tokens.map { |t| t.type.to_s }
end

# Helper: extract just the values from a token array.
def values(tokens)
  tokens.map(&:value)
end

class TestLatticeLexerVersion < Minitest::Test
  # The gem must expose a VERSION constant.
  def test_version_exists
    refute_nil CodingAdventures::LatticeLexer::VERSION
  end

  def test_version_is_string
    assert_kind_of String, CodingAdventures::LatticeLexer::VERSION
  end
end

class TestLatticeLexerInterface < Minitest::Test
  # Module-level .tokenize should return an Array.
  def test_tokenize_returns_array
    result = CodingAdventures::LatticeLexer.tokenize("h1 { color: red; }")
    assert_kind_of Array, result
  end

  # The array always ends with an EOF token.
  def test_tokenize_ends_with_eof
    result = CodingAdventures::LatticeLexer.tokenize("")
    assert_equal "EOF", result.last.type.to_s
  end

  # create_lexer returns a GrammarLexer object.
  def test_create_lexer_returns_lexer
    lexer = CodingAdventures::LatticeLexer.create_lexer("$x: 10px;")
    assert_respond_to lexer, :tokenize
  end

  # create_lexer result produces the same tokens as tokenize.
  def test_create_lexer_tokenize_consistent
    source = "$color: #fff;"
    expected = CodingAdventures::LatticeLexer.tokenize(source)
    lexer = CodingAdventures::LatticeLexer.create_lexer(source)
    result = lexer.tokenize
    assert_equal expected.map { |t| [t.type.to_s, t.value] },
      result.map { |t| [t.type.to_s, t.value] }
  end
end

class TestLatticeLexerSkipPatterns < Minitest::Test
  # Whitespace between tokens is skipped — no WHITESPACE tokens emitted.
  def test_whitespace_is_skipped
    toks = tokenize("  \t\r\n  ")
    assert_empty toks
  end

  # // line comments are skipped entirely.
  def test_line_comment_skipped
    toks = tokenize("// this is a comment\nh1 { color: red; }")
    assert_equal ["IDENT", "LBRACE", "IDENT", "COLON", "IDENT", "SEMICOLON", "RBRACE"],
      types(toks)
  end

  # /* block comments */ are skipped.
  def test_block_comment_skipped
    toks = tokenize("/* block comment */ h1 { color: red; }")
    assert_equal ["IDENT", "LBRACE", "IDENT", "COLON", "IDENT", "SEMICOLON", "RBRACE"],
      types(toks)
  end

  # Block comment spanning multiple lines.
  def test_multiline_block_comment_skipped
    toks = tokenize("/* line 1\n   line 2\n   line 3 */ h1 { }")
    type_list = types(toks)
    refute_includes type_list, "COMMENT"
  end
end

class TestLatticeLexerVariables < Minitest::Test
  # $identifier is a VARIABLE token. This is the core Lattice extension.
  def test_simple_variable
    toks = tokenize("$color")
    assert_equal ["VARIABLE"], types(toks)
    assert_equal ["$color"], values(toks)
  end

  # Variables with hyphens.
  def test_variable_with_hyphen
    toks = tokenize("$font-size")
    assert_equal ["VARIABLE"], types(toks)
    assert_equal ["$font-size"], values(toks)
  end

  # Variables with underscores.
  def test_variable_with_underscore
    toks = tokenize("$base_size")
    assert_equal ["VARIABLE"], types(toks)
    assert_equal ["$base_size"], values(toks)
  end

  # Variable in a declaration.
  def test_variable_in_declaration
    toks = tokenize("color: $brand-color;")
    assert_equal ["IDENT", "COLON", "VARIABLE", "SEMICOLON"], types(toks)
    assert_equal "$brand-color", toks[2].value
  end

  # Variable declaration.
  def test_variable_declaration
    toks = tokenize("$primary: #4a90d9;")
    assert_equal ["VARIABLE", "COLON", "HASH", "SEMICOLON"], types(toks)
    assert_equal "$primary", toks[0].value
  end
end

class TestLatticeLexerComparisonOperators < Minitest::Test
  # == equality operator (NEW in Lattice).
  def test_equals_equals
    toks = tokenize("==")
    assert_equal ["EQUALS_EQUALS"], types(toks)
    assert_equal ["=="], values(toks)
  end

  # != inequality operator (NEW in Lattice).
  def test_not_equals
    toks = tokenize("!=")
    assert_equal ["NOT_EQUALS"], types(toks)
    assert_equal ["!="], values(toks)
  end

  # >= greater-or-equal (NEW in Lattice).
  def test_greater_equals
    toks = tokenize(">=")
    assert_equal ["GREATER_EQUALS"], types(toks)
    assert_equal [">="], values(toks)
  end

  # <= less-or-equal (NEW in Lattice).
  def test_less_equals
    toks = tokenize("<=")
    assert_equal ["LESS_EQUALS"], types(toks)
    assert_equal ["<="], values(toks)
  end

  # @if expression uses == operator.
  def test_if_expression_with_equals
    toks = tokenize("$theme == dark")
    assert_equal ["VARIABLE", "EQUALS_EQUALS", "IDENT"], types(toks)
  end

  # Comparison in @if: $count >= 3
  def test_if_expression_with_greater_equals
    toks = tokenize("$count >= 3")
    assert_equal ["VARIABLE", "GREATER_EQUALS", "NUMBER"], types(toks)
  end
end

class TestLatticeLexerNumericTokens < Minitest::Test
  # Pure integer.
  def test_integer
    toks = tokenize("42")
    assert_equal ["NUMBER"], types(toks)
    assert_equal ["42"], values(toks)
  end

  # Floating point.
  def test_float
    toks = tokenize("3.14")
    assert_equal ["NUMBER"], types(toks)
    assert_equal ["3.14"], values(toks)
  end

  # Negative number.
  def test_negative_number
    toks = tokenize("-5")
    assert_equal ["NUMBER"], types(toks)
    assert_equal ["-5"], values(toks)
  end

  # DIMENSION: number + unit. ORDER: DIMENSION before NUMBER.
  def test_dimension_px
    toks = tokenize("16px")
    assert_equal ["DIMENSION"], types(toks)
    assert_equal ["16px"], values(toks)
  end

  def test_dimension_em
    toks = tokenize("2.5em")
    assert_equal ["DIMENSION"], types(toks)
    assert_equal ["2.5em"], values(toks)
  end

  def test_dimension_rem
    toks = tokenize("1rem")
    assert_equal ["DIMENSION"], types(toks)
    assert_equal ["1rem"], values(toks)
  end

  def test_dimension_vh
    toks = tokenize("100vh")
    assert_equal ["DIMENSION"], types(toks)
    assert_equal ["100vh"], values(toks)
  end

  # PERCENTAGE: number + %. ORDER: PERCENTAGE before NUMBER.
  def test_percentage
    toks = tokenize("50%")
    assert_equal ["PERCENTAGE"], types(toks)
    assert_equal ["50%"], values(toks)
  end

  def test_negative_percentage
    toks = tokenize("-10%")
    assert_equal ["PERCENTAGE"], types(toks)
    assert_equal ["-10%"], values(toks)
  end
end

class TestLatticeLexerStringTokens < Minitest::Test
  # Double-quoted string → STRING token.
  def test_double_quoted_string
    toks = tokenize('"hello world"')
    assert_equal ["STRING"], types(toks)
    assert_equal ["hello world"], values(toks)
  end

  # Single-quoted string → STRING token.
  def test_single_quoted_string
    toks = tokenize("'hello world'")
    assert_equal ["STRING"], types(toks)
    assert_equal ["hello world"], values(toks)
  end

  # String with escaped quote.
  def test_string_with_escape
    toks = tokenize('"it\\"s"')
    assert_equal ["STRING"], types(toks)
  end
end

class TestLatticeLexerIdentifiers < Minitest::Test
  # Plain identifier.
  def test_ident
    toks = tokenize("color")
    assert_equal ["IDENT"], types(toks)
    assert_equal ["color"], values(toks)
  end

  # Hyphenated identifier (CSS custom property prefix).
  def test_hyphenated_ident
    toks = tokenize("font-size")
    # font-size is IDENT followed by MINUS IDENT, or just IDENT depending
    # on grammar priority. In Lattice, "font-size" is one IDENT.
    # The grammar regex /-?[a-zA-Z_][a-zA-Z0-9_-]*/ matches the whole thing.
    assert_includes ["IDENT"], types(toks).first
  end

  # CSS custom property: --variable-name.
  def test_custom_property
    toks = tokenize("--my-color")
    assert_equal ["CUSTOM_PROPERTY"], types(toks)
    assert_equal ["--my-color"], values(toks)
  end

  # @at-keyword for Lattice directives.
  def test_at_keyword_mixin
    toks = tokenize("@mixin")
    assert_equal ["AT_KEYWORD"], types(toks)
    assert_equal ["@mixin"], values(toks)
  end

  def test_at_keyword_if
    toks = tokenize("@if")
    assert_equal ["AT_KEYWORD"], types(toks)
    assert_equal ["@if"], values(toks)
  end

  def test_at_keyword_function
    toks = tokenize("@function")
    assert_equal ["AT_KEYWORD"], types(toks)
  end

  def test_at_keyword_use
    toks = tokenize("@use")
    assert_equal ["AT_KEYWORD"], types(toks)
  end
end

class TestLatticeLexerHashAndColor < Minitest::Test
  # HASH token: #hex-color or #id-selector.
  def test_hash_color
    toks = tokenize("#4a90d9")
    assert_equal ["HASH"], types(toks)
    assert_equal ["#4a90d9"], values(toks)
  end

  def test_hash_short_color
    toks = tokenize("#fff")
    assert_equal ["HASH"], types(toks)
    assert_equal ["#fff"], values(toks)
  end

  def test_hash_id_selector
    toks = tokenize("#my-id")
    assert_equal ["HASH"], types(toks)
    assert_equal ["#my-id"], values(toks)
  end
end

class TestLatticeLexerFunctionTokens < Minitest::Test
  # FUNCTION token: name followed by ( — includes the paren.
  def test_function_token_rgb
    toks = tokenize("rgb(")
    assert_equal ["FUNCTION"], types(toks)
    assert_equal ["rgb("], values(toks)
  end

  def test_function_token_calc
    toks = tokenize("calc(")
    assert_equal ["FUNCTION"], types(toks)
    assert_equal ["calc("], values(toks)
  end

  def test_function_token_custom
    toks = tokenize("spacing(")
    assert_equal ["FUNCTION"], types(toks)
    assert_equal ["spacing("], values(toks)
  end

  # URL token: url(path-without-quotes).
  def test_url_token
    toks = tokenize("url(https://example.com/img.png)")
    assert_equal ["URL_TOKEN"], types(toks)
  end
end

class TestLatticeLexerDelimiters < Minitest::Test
  # All single-character delimiter tokens.
  DELIMITERS = [
    ["{", "LBRACE"], ["}", "RBRACE"], ["(", "LPAREN"], [")", "RPAREN"],
    ["[", "LBRACKET"], ["]", "RBRACKET"], [";", "SEMICOLON"], [":", "COLON"],
    [",", "COMMA"], [".", "DOT"], ["+", "PLUS"], [">", "GREATER"],
    ["~", "TILDE"], ["*", "STAR"], ["|", "PIPE"], ["!", "BANG"],
    ["/", "SLASH"], ["=", "EQUALS"], ["&", "AMPERSAND"], ["-", "MINUS"]
  ].freeze

  DELIMITERS.each do |char, expected_type|
    define_method("test_delimiter_#{expected_type.downcase}") do
      toks = tokenize(char)
      assert_equal [expected_type], types(toks),
        "Expected #{char.inspect} to produce #{expected_type}"
    end
  end

  # Multi-character CSS attribute selectors.
  def test_colon_colon
    toks = tokenize("::")
    assert_equal ["COLON_COLON"], types(toks)
  end

  def test_tilde_equals
    toks = tokenize("~=")
    assert_equal ["TILDE_EQUALS"], types(toks)
  end

  def test_pipe_equals
    toks = tokenize("|=")
    assert_equal ["PIPE_EQUALS"], types(toks)
  end

  def test_caret_equals
    toks = tokenize("^=")
    assert_equal ["CARET_EQUALS"], types(toks)
  end

  def test_dollar_equals
    toks = tokenize("$=")
    assert_equal ["DOLLAR_EQUALS"], types(toks)
  end

  def test_star_equals
    toks = tokenize("*=")
    assert_equal ["STAR_EQUALS"], types(toks)
  end
end

class TestLatticeLexerIntegration < Minitest::Test
  # Variable declaration: $primary: #4a90d9;
  def test_variable_declaration
    toks = tokenize("$primary: #4a90d9;")
    assert_equal ["VARIABLE", "COLON", "HASH", "SEMICOLON"], types(toks)
  end

  # Simple CSS rule: h1 { color: red; }
  def test_css_rule
    toks = tokenize("h1 { color: red; }")
    assert_equal ["IDENT", "LBRACE", "IDENT", "COLON", "IDENT", "SEMICOLON", "RBRACE"],
      types(toks)
  end

  # @mixin definition.
  def test_mixin_definition_tokens
    toks = tokenize("@mixin button($bg) { background: $bg; }")
    type_list = types(toks)
    assert_includes type_list, "AT_KEYWORD"
    assert_includes type_list, "FUNCTION"
    assert_includes type_list, "VARIABLE"
  end

  # @if expression: @if $theme == dark { ... }
  def test_if_directive_tokens
    toks = tokenize("@if $theme == dark { color: white; }")
    type_list = types(toks)
    assert_equal "AT_KEYWORD", type_list[0]
    assert_equal "VARIABLE", type_list[1]
    assert_equal "EQUALS_EQUALS", type_list[2]
    assert_equal "IDENT", type_list[3]
  end

  # @for loop: @for $i from 1 through 3 { }
  def test_for_loop_tokens
    toks = tokenize("@for $i from 1 through 3 { }")
    type_list = types(toks)
    assert_equal "AT_KEYWORD", type_list[0]
    assert_equal "VARIABLE", type_list[1]
    assert_equal "IDENT", type_list[2]  # "from"
    assert_equal "NUMBER", type_list[3]
  end

  # @each: @each $color in red, green, blue { }
  def test_each_loop_tokens
    toks = tokenize("@each $color in red, green, blue { }")
    type_list = types(toks)
    assert_equal "AT_KEYWORD", type_list[0]
    assert_equal "VARIABLE", type_list[1]
    assert_equal "IDENT", type_list[2]  # "in"
  end

  # @use module import: @use "colors";
  def test_use_directive_tokens
    toks = tokenize('@use "colors";')
    assert_equal ["AT_KEYWORD", "STRING", "SEMICOLON"], types(toks)
    assert_equal "@use", toks[0].value
  end

  # @function definition: @function spacing($n) { @return $n * 8px; }
  def test_function_definition_tokens
    toks = tokenize("@function spacing($n) { @return $n * 8px; }")
    type_list = types(toks)
    assert_equal "AT_KEYWORD", type_list[0]
    assert_equal "@function", toks[0].value
    assert_includes type_list, "VARIABLE"
    assert_includes type_list, "DIMENSION"
  end

  # Include directive: @include button(red);
  def test_include_directive_tokens
    toks = tokenize("@include button(red);")
    type_list = types(toks)
    assert_equal "AT_KEYWORD", type_list[0]
    assert_equal "@include", toks[0].value
    assert_equal "FUNCTION", type_list[1]
    assert_equal "IDENT", type_list[2]
  end

  # Multiple declarations inside a rule.
  def test_multi_declaration_rule
    source = "h1 { color: red; font-size: 16px; margin: 0; }"
    toks = tokenize(source)
    assert_equal 3, toks.count { |t| t.type.to_s == "SEMICOLON" }
  end

  # Complex Lattice source with variables and a rule.
  def test_variable_substitution_context
    source = <<~LATTICE
      $primary: #4a90d9;
      h1 { color: $primary; }
    LATTICE
    toks = tokenize(source)
    type_list = types(toks)
    assert_equal 2, type_list.count("VARIABLE")
    assert_equal 1, type_list.count("HASH")
  end

  # CSS class selector with dot.
  def test_class_selector
    toks = tokenize(".btn { padding: 8px; }")
    type_list = types(toks)
    assert_equal "DOT", type_list[0]
    assert_equal "IDENT", type_list[1]
  end

  # Pseudo-class: a:hover
  def test_pseudo_class
    toks = tokenize("a:hover")
    assert_equal ["IDENT", "COLON", "IDENT"], types(toks)
  end

  # Pseudo-element: p::before
  def test_pseudo_element
    toks = tokenize("p::before")
    assert_equal ["IDENT", "COLON_COLON", "IDENT"], types(toks)
  end

  # @media at-rule prelude.
  def test_media_query
    toks = tokenize("@media (max-width: 768px)")
    assert_equal "AT_KEYWORD", toks[0].type.to_s
    assert_equal "@media", toks[0].value
  end

  # !important priority.
  def test_important
    toks = tokenize("color: red !important;")
    type_list = types(toks)
    assert_includes type_list, "BANG"
    assert_includes type_list, "IDENT"
  end
end
