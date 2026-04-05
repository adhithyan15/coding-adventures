# frozen_string_literal: true

# ================================================================
# Tests for the Mosaic Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with mosaic.tokens, correctly tokenizes Mosaic source text.
#
# Mosaic is a Component Description Language (CDL) that compiles
# to UI platform code (React, Web Components, SwiftUI, etc.). Its
# token vocabulary includes:
#
#   Literals:   STRING, NUMBER, DIMENSION, COLOR_HEX
#   Keywords:   component, slot, import, from, as, text, number,
#               bool, image, color, node, list, true, false, when, each
#   Identifier: NAME (allows hyphens for CSS-like names)
#   Delimiters: LBRACE, RBRACE, LANGLE, RANGLE, COLON, SEMICOLON,
#               COMMA, DOT, EQUALS, AT
#   Skipped:    LINE_COMMENT, BLOCK_COMMENT, WHITESPACE
# ================================================================

require "minitest/autorun"
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_mosaic_lexer"

class TestMosaicLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  # Token type constants not in TokenType::ALL (Mosaic-specific)
  AT_TYPE       = "AT"
  LANGLE_TYPE   = "LANGLE"
  RANGLE_TYPE   = "RANGLE"
  DIMENSION_TYPE = "DIMENSION"
  COLOR_HEX_TYPE = "COLOR_HEX"
  KEYWORD_TYPE  = "KEYWORD"
  NUMBER_TYPE   = "NUMBER"

  # ------------------------------------------------------------------
  # Helper methods
  # ------------------------------------------------------------------

  def tokenize(source)
    CodingAdventures::MosaicLexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  def non_eof(tokens)
    tokens.reject { |t| t.type == TT::EOF }
  end

  # ------------------------------------------------------------------
  # Version and grammar path
  # ------------------------------------------------------------------

  def test_version_exists
    refute_nil CodingAdventures::MosaicLexer::VERSION
  end

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::MosaicLexer::MOSAIC_TOKENS_PATH),
      "mosaic.tokens should exist at #{CodingAdventures::MosaicLexer::MOSAIC_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Basic EOF
  # ------------------------------------------------------------------

  def test_empty_input_produces_eof
    tokens = tokenize("")
    assert_equal TT::EOF, tokens.last.type
  end

  def test_whitespace_only_produces_eof
    tokens = tokenize("   \n\t  ")
    assert_equal [TT::EOF], tokens.map(&:type)
  end

  # ------------------------------------------------------------------
  # Keywords
  # ------------------------------------------------------------------

  def test_keyword_component
    tokens = non_eof(tokenize("component"))
    assert_equal 1, tokens.length
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "component", tokens[0].value
  end

  def test_keyword_slot
    tokens = non_eof(tokenize("slot"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "slot", tokens[0].value
  end

  def test_keyword_import
    tokens = non_eof(tokenize("import"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "import", tokens[0].value
  end

  def test_keyword_from
    tokens = non_eof(tokenize("from"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "from", tokens[0].value
  end

  def test_keyword_as
    tokens = non_eof(tokenize("as"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "as", tokens[0].value
  end

  def test_keyword_text
    tokens = non_eof(tokenize("text"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "text", tokens[0].value
  end

  def test_keyword_bool
    tokens = non_eof(tokenize("bool"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "bool", tokens[0].value
  end

  def test_keyword_list
    tokens = non_eof(tokenize("list"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "list", tokens[0].value
  end

  def test_keyword_true
    tokens = non_eof(tokenize("true"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "true", tokens[0].value
  end

  def test_keyword_false
    tokens = non_eof(tokenize("false"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "false", tokens[0].value
  end

  def test_keyword_when
    tokens = non_eof(tokenize("when"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "when", tokens[0].value
  end

  def test_keyword_each
    tokens = non_eof(tokenize("each"))
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "each", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Identifiers (NAME)
  # ------------------------------------------------------------------

  def test_name_simple
    tokens = non_eof(tokenize("ProfileCard"))
    assert_equal TT::NAME, tokens[0].type
    assert_equal "ProfileCard", tokens[0].value
  end

  def test_name_with_hyphen
    # Mosaic allows hyphens in property names like corner-radius
    tokens = non_eof(tokenize("corner-radius"))
    assert_equal TT::NAME, tokens[0].type
    assert_equal "corner-radius", tokens[0].value
  end

  def test_name_with_digits
    tokens = non_eof(tokenize("item1"))
    assert_equal TT::NAME, tokens[0].type
    assert_equal "item1", tokens[0].value
  end

  def test_name_underscore
    tokens = non_eof(tokenize("_private"))
    assert_equal TT::NAME, tokens[0].type
    assert_equal "_private", tokens[0].value
  end

  # ------------------------------------------------------------------
  # String literals
  # ------------------------------------------------------------------

  def test_string_literal
    tokens = non_eof(tokenize('"hello"'))
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_string_path
    tokens = non_eof(tokenize('"./button.mosaic"'))
    assert_equal TT::STRING, tokens[0].type
    assert_equal "./button.mosaic", tokens[0].value
  end

  def test_string_with_spaces
    tokens = non_eof(tokenize('"Hello, World!"'))
    assert_equal TT::STRING, tokens[0].type
    assert_equal "Hello, World!", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Number literals
  # ------------------------------------------------------------------

  def test_integer
    tokens = non_eof(tokenize("42"))
    assert_equal "NUMBER", tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_negative_number
    tokens = non_eof(tokenize("-1"))
    assert_equal "NUMBER", tokens[0].type
    assert_equal "-1", tokens[0].value
  end

  def test_float
    tokens = non_eof(tokenize("3.14"))
    assert_equal "NUMBER", tokens[0].type
    assert_equal "3.14", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Dimension literals (number + unit)
  # ------------------------------------------------------------------

  def test_dimension_dp
    tokens = non_eof(tokenize("16dp"))
    assert_equal "DIMENSION", tokens[0].type
    assert_equal "16dp", tokens[0].value
  end

  def test_dimension_sp
    tokens = non_eof(tokenize("14sp"))
    assert_equal "DIMENSION", tokens[0].type
    assert_equal "14sp", tokens[0].value
  end

  def test_dimension_percent
    tokens = non_eof(tokenize("100%"))
    assert_equal "DIMENSION", tokens[0].type
    assert_equal "100%", tokens[0].value
  end

  def test_dimension_before_number
    # DIMENSION has higher priority than NUMBER when a unit suffix follows
    tokens = non_eof(tokenize("16dp"))
    assert_equal "DIMENSION", tokens[0].type
  end

  # ------------------------------------------------------------------
  # Color hex literals
  # ------------------------------------------------------------------

  def test_color_6_digit
    tokens = non_eof(tokenize("#2563eb"))
    assert_equal "COLOR_HEX", tokens[0].type
    assert_equal "#2563eb", tokens[0].value
  end

  def test_color_3_digit
    tokens = non_eof(tokenize("#fff"))
    assert_equal "COLOR_HEX", tokens[0].type
    assert_equal "#fff", tokens[0].value
  end

  def test_color_8_digit
    tokens = non_eof(tokenize("#2563eb80"))
    assert_equal "COLOR_HEX", tokens[0].type
    assert_equal "#2563eb80", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Delimiter tokens
  # ------------------------------------------------------------------

  def test_lbrace
    tokens = non_eof(tokenize("{"))
    assert_equal TT::LBRACE, tokens[0].type
  end

  def test_rbrace
    tokens = non_eof(tokenize("}"))
    assert_equal TT::RBRACE, tokens[0].type
  end

  def test_colon
    tokens = non_eof(tokenize(":"))
    assert_equal TT::COLON, tokens[0].type
  end

  def test_semicolon
    tokens = non_eof(tokenize(";"))
    assert_equal TT::SEMICOLON, tokens[0].type
  end

  def test_at
    tokens = non_eof(tokenize("@"))
    assert_equal AT_TYPE, tokens[0].type
  end

  def test_equals
    tokens = non_eof(tokenize("="))
    assert_equal TT::EQUALS, tokens[0].type
  end

  def test_langle
    tokens = non_eof(tokenize("<"))
    assert_equal LANGLE_TYPE, tokens[0].type
  end

  def test_rangle
    tokens = non_eof(tokenize(">"))
    assert_equal RANGLE_TYPE, tokens[0].type
  end

  def test_comma
    tokens = non_eof(tokenize(","))
    assert_equal TT::COMMA, tokens[0].type
  end

  def test_dot
    tokens = non_eof(tokenize("."))
    assert_equal TT::DOT, tokens[0].type
  end

  # ------------------------------------------------------------------
  # Comments are skipped
  # ------------------------------------------------------------------

  def test_line_comment_skipped
    tokens = non_eof(tokenize("// this is a comment\nfoo"))
    assert_equal 1, tokens.length
    assert_equal TT::NAME, tokens[0].type
    assert_equal "foo", tokens[0].value
  end

  def test_block_comment_skipped
    tokens = non_eof(tokenize("/* block */ foo"))
    assert_equal 1, tokens.length
    assert_equal "foo", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Slot reference pattern: @slotName
  # ------------------------------------------------------------------

  def test_slot_reference
    tokens = non_eof(tokenize("@title"))
    types = tokens.map(&:type)
    assert_equal [AT_TYPE, TT::NAME], types
    assert_equal "title", tokens[1].value
  end

  def test_slot_reference_with_hyphen
    tokens = non_eof(tokenize("@avatar-url"))
    types = tokens.map(&:type)
    assert_equal [AT_TYPE, TT::NAME], types
    assert_equal "avatar-url", tokens[1].value
  end

  # ------------------------------------------------------------------
  # A complete minimal component
  # ------------------------------------------------------------------

  def test_minimal_component
    source = 'component Label { slot title: text; Text { content: @title; } }'
    tokens = non_eof(tokenize(source))
    # First two tokens should be KEYWORD(component) NAME(Label)
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "component", tokens[0].value
    assert_equal TT::NAME, tokens[1].type
    assert_equal "Label", tokens[1].value
  end

  def test_slot_declaration_tokens
    source = "slot count: number = 0;"
    tokens = non_eof(tokenize(source))
    types = tokens.map(&:type)
    # slot(KW) count(NAME) :(COLON) number(KW) =(EQUALS) 0(NUMBER) ;(SEMICOLON)
    assert_includes types, "KEYWORD"
    assert_includes types, TT::NAME
    assert_includes types, TT::COLON
    assert_includes types, TT::SEMICOLON
  end

  def test_list_type_tokens
    source = "list<text>"
    tokens = non_eof(tokenize(source))
    types = tokens.map(&:type)
    # list(KW) <(LANGLE) text(KW) >(RANGLE)
    assert_equal "KEYWORD", types[0]
    assert_equal LANGLE_TYPE, types[1]
    assert_equal "KEYWORD", types[2]
    assert_equal RANGLE_TYPE, types[3]
  end

  def test_import_declaration_tokens
    source = 'import Button from "./button.mosaic";'
    tokens = non_eof(tokenize(source))
    # import(KW) Button(NAME) from(KW) "./button.mosaic"(STRING) ;(SEMICOLON)
    assert_equal "KEYWORD", tokens[0].type
    assert_equal "import", tokens[0].value
    assert_equal TT::NAME, tokens[1].type
    assert_equal "Button", tokens[1].value
    assert_equal "KEYWORD", tokens[2].type
    assert_equal "from", tokens[2].value
    assert_equal TT::STRING, tokens[3].type
    assert_equal TT::SEMICOLON, tokens[4].type
  end

  def test_enum_value_tokens
    # style: heading.medium  -- dot-separated enum
    source = "heading.medium"
    tokens = non_eof(tokenize(source))
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::DOT, TT::NAME], types
  end

  def test_line_number_tracking
    source = "component\nLabel"
    tokens = tokenize(source)
    name_token = tokens.find { |t| t.type == TT::NAME && t.value == "Label" }
    refute_nil name_token
    assert_equal 2, name_token.line
  end
end
