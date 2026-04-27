# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Verilog Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with verilog.tokens, correctly tokenizes Verilog HDL source code.
#
# Verilog has several unique token types not found in software
# languages: sized numbers (4'b1010), system tasks ($display),
# compiler directives (`define), and escaped identifiers (\bus[0]).
# ================================================================

class TestVerilogLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  def tokenize(source, preprocess: false)
    CodingAdventures::VerilogLexer.tokenize(source, preprocess: preprocess)
  end

  def token_types(source, preprocess: false)
    tokenize(source, preprocess: preprocess).map(&:type)
  end

  def token_values(source, preprocess: false)
    tokenize(source, preprocess: preprocess).map(&:value)
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::VerilogLexer::VERILOG_TOKENS_PATH),
      "verilog.tokens file should exist at #{CodingAdventures::VerilogLexer::VERILOG_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Basic wire declaration: wire [7:0] data;
  # ------------------------------------------------------------------

  def test_wire_declaration
    tokens = tokenize("wire data;")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::NAME, TT::SEMICOLON, TT::EOF], types
  end

  def test_wire_declaration_values
    tokens = tokenize("wire data;")
    values = tokens.map(&:value)
    assert_equal ["wire", "data", ";", ""], values
  end

  # ------------------------------------------------------------------
  # Keywords
  # ------------------------------------------------------------------

  def test_keyword_module
    tokens = tokenize("module")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "module", tokens[0].value
  end

  def test_keyword_endmodule
    tokens = tokenize("endmodule")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "endmodule", tokens[0].value
  end

  def test_keyword_assign
    tokens = tokenize("assign")
    assert_equal TT::KEYWORD, tokens[0].type
  end

  def test_keyword_always
    tokens = tokenize("always")
    assert_equal TT::KEYWORD, tokens[0].type
  end

  def test_keyword_input_output
    tokens = tokenize("input output inout")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[input output inout], keywords
  end

  def test_keyword_reg_wire_integer
    tokens = tokenize("reg wire integer")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[reg wire integer], keywords
  end

  def test_keyword_if_else_case
    tokens = tokenize("if else case endcase default")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[if else case endcase default], keywords
  end

  def test_keyword_begin_end
    tokens = tokenize("begin end")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[begin end], keywords
  end

  def test_keyword_posedge_negedge
    tokens = tokenize("posedge negedge")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[posedge negedge], keywords
  end

  def test_keyword_generate
    tokens = tokenize("generate endgenerate genvar")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[generate endgenerate genvar], keywords
  end

  def test_keyword_function_task
    tokens = tokenize("function endfunction task endtask")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[function endfunction task endtask], keywords
  end

  def test_keyword_parameter_localparam
    tokens = tokenize("parameter localparam")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[parameter localparam], keywords
  end

  def test_keyword_gate_primitives
    tokens = tokenize("and nand or nor xor xnor not buf")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[and nand or nor xor xnor not buf], keywords
  end

  def test_name_not_keyword
    tokens = tokenize("counter")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "counter", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Numbers -- plain integers
  # ------------------------------------------------------------------

  def test_plain_number
    tokens = tokenize("42")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_number_with_underscores
    tokens = tokenize("1_000_000")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "1_000_000", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Sized numbers -- hardware-specific bit-width notation
  # ------------------------------------------------------------------

  def test_sized_binary_number
    tokens = tokenize("4'b1010")
    assert_equal "4'b1010", tokens[0].value
  end

  def test_sized_hex_number
    tokens = tokenize("8'hFF")
    assert_equal "8'hFF", tokens[0].value
  end

  def test_sized_decimal_number
    tokens = tokenize("32'd42")
    assert_equal "32'd42", tokens[0].value
  end

  def test_unsized_octal
    tokens = tokenize("'o77")
    assert_equal "'o77", tokens[0].value
  end

  def test_sized_with_underscores
    tokens = tokenize("8'b1010_0011")
    assert_equal "8'b1010_0011", tokens[0].value
  end

  def test_sized_with_xz
    tokens = tokenize("4'bxxzz")
    assert_equal "4'bxxzz", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Real numbers
  # ------------------------------------------------------------------

  def test_real_number
    tokens = tokenize("3.14")
    assert_equal "3.14", tokens[0].value
  end

  def test_real_with_exponent
    tokens = tokenize("1.5e-3")
    assert_equal "1.5e-3", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Strings
  # ------------------------------------------------------------------

  def test_string_literal
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Special identifiers
  # ------------------------------------------------------------------

  def test_system_task
    tokens = tokenize("$display")
    assert_equal "$display", tokens[0].value
  end

  def test_system_task_time
    tokens = tokenize("$time")
    assert_equal "$time", tokens[0].value
  end

  def test_directive
    tokens = tokenize("`define")
    assert_equal "`define", tokens[0].value
  end

  def test_escaped_identifier
    tokens = tokenize('\my_name ')
    assert_equal '\\my_name', tokens[0].value
  end

  # ------------------------------------------------------------------
  # Three-character operators
  # ------------------------------------------------------------------

  def test_arithmetic_left_shift
    tokens = tokenize("a <<< 2")
    assert_equal "<<<", tokens[1].value
  end

  def test_arithmetic_right_shift
    tokens = tokenize("a >>> 2")
    assert_equal ">>>", tokens[1].value
  end

  def test_case_equality
    tokens = tokenize("a === b")
    assert_equal "===", tokens[1].value
  end

  def test_case_inequality
    tokens = tokenize("a !== b")
    assert_equal "!==", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Two-character operators
  # ------------------------------------------------------------------

  def test_logic_and
    tokens = tokenize("a && b")
    assert_equal "&&", tokens[1].value
  end

  def test_logic_or
    tokens = tokenize("a || b")
    assert_equal "||", tokens[1].value
  end

  def test_left_shift
    tokens = tokenize("a << 1")
    assert_equal "<<", tokens[1].value
  end

  def test_right_shift
    tokens = tokenize("a >> 1")
    assert_equal ">>", tokens[1].value
  end

  def test_equals_equals
    tokens = tokenize("a == b")
    types = tokens.map(&:type)
    assert_equal [TT::NAME, TT::EQUALS_EQUALS, TT::NAME, TT::EOF], types
  end

  def test_not_equals
    tokens = tokenize("a != b")
    assert_equal "!=", tokens[1].value
  end

  def test_less_equals
    tokens = tokenize("a <= b")
    assert_equal "<=", tokens[1].value
  end

  def test_greater_equals
    tokens = tokenize("a >= b")
    assert_equal ">=", tokens[1].value
  end

  def test_power_operator
    tokens = tokenize("a ** b")
    assert_equal "**", tokens[1].value
  end

  def test_trigger_operator
    tokens = tokenize("->")
    assert_equal "->", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Single-character operators
  # ------------------------------------------------------------------

  def test_arithmetic_operators
    tokens = tokenize("+ - * / %")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_equal ["+", "-", "*", "/", "%"], values
  end

  def test_bitwise_operators
    tokens = tokenize("& | ^ ~")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_equal ["&", "|", "^", "~"], values
  end

  def test_bang_operator
    tokens = tokenize("!")
    assert_equal "!", tokens[0].value
  end

  def test_question_colon
    tokens = tokenize("? :")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_equal ["?", ":"], values
  end

  # ------------------------------------------------------------------
  # Delimiters
  # ------------------------------------------------------------------

  def test_parens
    tokens = tokenize("( )")
    types = tokens.map(&:type)
    assert_equal [TT::LPAREN, TT::RPAREN, TT::EOF], types
  end

  def test_brackets
    tokens = tokenize("[ ]")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACKET, TT::RBRACKET, TT::EOF], types
  end

  def test_braces
    tokens = tokenize("{ }")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACE, TT::RBRACE, TT::EOF], types
  end

  def test_semicolon
    tokens = tokenize(";")
    assert_equal TT::SEMICOLON, tokens[0].type
  end

  def test_comma
    tokens = tokenize(",")
    assert_equal ",", tokens[0].value
  end

  def test_dot
    tokens = tokenize(".")
    assert_equal ".", tokens[0].value
  end

  def test_hash_delimiter
    tokens = tokenize("#")
    assert_equal "#", tokens[0].value
  end

  def test_at_delimiter
    tokens = tokenize("@")
    assert_equal "@", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Module declaration (realistic example)
  # ------------------------------------------------------------------

  def test_module_declaration
    source = "module counter(input clk, input rst, output [7:0] count);"
    tokens = tokenize(source)
    # Should tokenize without errors and produce reasonable tokens
    assert tokens.length > 10
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "module", tokens[0].value
    assert_equal TT::EOF, tokens.last.type
  end

  # ------------------------------------------------------------------
  # Always block (realistic example)
  # ------------------------------------------------------------------

  def test_always_block
    source = "always @(posedge clk) begin q <= d; end"
    tokens = tokenize(source)
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_includes keywords, "always"
    assert_includes keywords, "posedge"
    assert_includes keywords, "begin"
    assert_includes keywords, "end"
  end

  # ------------------------------------------------------------------
  # Preprocessing integration
  # ------------------------------------------------------------------

  def test_tokenize_with_preprocess_option
    source = "`define WIDTH 8\nwire [WIDTH-1:0] data;"
    # Without preprocess, `define is a directive token
    tokens_raw = tokenize(source, preprocess: false)
    raw_values = tokens_raw.map(&:value)
    assert_includes raw_values, "`define"
  end

  def test_tokenize_with_preprocess_enabled
    source = "`define WIDTH 8\nwire [`WIDTH-1:0] data;"
    tokens = tokenize(source, preprocess: true)
    values = tokens.map(&:value)
    # After preprocessing, `WIDTH should be expanded to 8
    assert_includes values, "8"
    refute values.any? { |v| v == "`define" }
  end

  # ------------------------------------------------------------------
  # Comments (should be skipped)
  # ------------------------------------------------------------------

  def test_line_comment_skipped
    tokens = tokenize("wire a; // this is a comment")
    types = tokens.map(&:type)
    refute_includes types, :LINE_COMMENT
    assert_equal [TT::KEYWORD, TT::NAME, TT::SEMICOLON, TT::EOF], types
  end

  def test_block_comment_skipped
    tokens = tokenize("wire /* block */ a;")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::NAME, TT::SEMICOLON, TT::EOF], types
  end

  def test_default_version_matches_explicit_2005
    default_tokens = tokenize("module m; endmodule")
    explicit_tokens = CodingAdventures::VerilogLexer.tokenize(
      "module m; endmodule",
      version: "2005"
    )
    assert_equal default_tokens.map(&:value), explicit_tokens.map(&:value)
  end

  def test_rejects_unknown_version
    error = assert_raises(ArgumentError) do
      CodingAdventures::VerilogLexer.tokenize("module m; endmodule", version: "2099")
    end
    assert_match(/Unknown Verilog version/, error.message)
  end
end
