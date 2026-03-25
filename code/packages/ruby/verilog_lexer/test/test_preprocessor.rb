# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Verilog Preprocessor
# ================================================================
#
# The preprocessor resolves Verilog compiler directives before the
# source is passed to the lexer. These tests verify each directive
# type and their interactions (e.g., nested `ifdef blocks).
# ================================================================

class TestPreprocessor < Minitest::Test
  PP = CodingAdventures::VerilogLexer::Preprocessor

  def preprocess(source)
    PP.process(source)
  end

  # ------------------------------------------------------------------
  # `define and simple macro expansion
  # ------------------------------------------------------------------

  def test_simple_define_and_expansion
    source = "`define WIDTH 8\nwire [`WIDTH-1:0] data;"
    result = preprocess(source)
    assert_equal "wire [8-1:0] data;", result
  end

  def test_define_multiple_macros
    source = "`define A 10\n`define B 20\nassign x = `A + `B;"
    result = preprocess(source)
    assert_equal "assign x = 10 + 20;", result
  end

  def test_define_flag_macro_no_value
    source = "`define USE_CACHE\n`ifdef USE_CACHE\nwire cache;\n`endif"
    result = preprocess(source)
    assert_equal "wire cache;", result
  end

  def test_macro_not_expanded_when_undefined
    source = "wire `UNKNOWN;"
    result = preprocess(source)
    assert_equal "wire `UNKNOWN;", result
  end

  # ------------------------------------------------------------------
  # `undef
  # ------------------------------------------------------------------

  def test_undef_removes_macro
    source = "`define WIDTH 8\n`undef WIDTH\nwire [`WIDTH:0] data;"
    result = preprocess(source)
    # After undef, `WIDTH should not expand
    assert_includes result, "`WIDTH"
  end

  # ------------------------------------------------------------------
  # Parameterized macros
  # ------------------------------------------------------------------

  def test_parameterized_macro
    source = "`define MAX(a, b) ((a) > (b) ? (a) : (b))\nassign out = `MAX(x, y);"
    result = preprocess(source)
    assert_equal "assign out = ((x) > (y) ? (x) : (y));", result
  end

  def test_parameterized_macro_nested_parens
    source = "`define ADD(a, b) ((a) + (b))\nassign out = `ADD((x+1), (y+2));"
    result = preprocess(source)
    assert_equal "assign out = (((x+1)) + ((y+2)));", result
  end

  def test_parameterized_macro_single_param
    source = "`define NEG(x) (-(x))\nassign out = `NEG(val);"
    result = preprocess(source)
    assert_equal "assign out = (-(val));", result
  end

  # ------------------------------------------------------------------
  # `ifdef / `endif
  # ------------------------------------------------------------------

  def test_ifdef_defined
    source = "`define DEBUG\n`ifdef DEBUG\nwire debug_out;\n`endif"
    result = preprocess(source)
    assert_equal "wire debug_out;", result
  end

  def test_ifdef_not_defined
    source = "`ifdef DEBUG\nwire debug_out;\n`endif"
    result = preprocess(source)
    assert_equal "", result
  end

  # ------------------------------------------------------------------
  # `ifndef / `endif
  # ------------------------------------------------------------------

  def test_ifndef_not_defined
    source = "`ifndef GUARD\nwire data;\n`endif"
    result = preprocess(source)
    assert_equal "wire data;", result
  end

  def test_ifndef_defined
    source = "`define GUARD\n`ifndef GUARD\nwire data;\n`endif"
    result = preprocess(source)
    assert_equal "", result
  end

  # ------------------------------------------------------------------
  # `ifdef / `else / `endif
  # ------------------------------------------------------------------

  def test_ifdef_else_when_defined
    source = "`define USE_A\n`ifdef USE_A\nwire a;\n`else\nwire b;\n`endif"
    result = preprocess(source)
    assert_equal "wire a;", result
  end

  def test_ifdef_else_when_not_defined
    source = "`ifdef USE_A\nwire a;\n`else\nwire b;\n`endif"
    result = preprocess(source)
    assert_equal "wire b;", result
  end

  # ------------------------------------------------------------------
  # `ifndef / `else / `endif
  # ------------------------------------------------------------------

  def test_ifndef_else_when_not_defined
    source = "`ifndef FEATURE\nwire fallback;\n`else\nwire feature;\n`endif"
    result = preprocess(source)
    assert_equal "wire fallback;", result
  end

  def test_ifndef_else_when_defined
    source = "`define FEATURE\n`ifndef FEATURE\nwire fallback;\n`else\nwire feature;\n`endif"
    result = preprocess(source)
    assert_equal "wire feature;", result
  end

  # ------------------------------------------------------------------
  # Nested conditionals
  # ------------------------------------------------------------------

  def test_nested_ifdef
    source = [
      "`define OUTER",
      "`define INNER",
      "`ifdef OUTER",
      "wire outer;",
      "`ifdef INNER",
      "wire inner;",
      "`endif",
      "`endif"
    ].join("\n")
    result = preprocess(source)
    assert_includes result, "wire outer;"
    assert_includes result, "wire inner;"
  end

  def test_nested_ifdef_outer_false
    source = [
      "`define INNER",
      "`ifdef OUTER",
      "wire outer;",
      "`ifdef INNER",
      "wire inner;",
      "`endif",
      "`endif"
    ].join("\n")
    result = preprocess(source)
    refute_includes result, "wire outer;"
    refute_includes result, "wire inner;"
  end

  # ------------------------------------------------------------------
  # `include (stubbed)
  # ------------------------------------------------------------------

  def test_include_stubbed
    source = '`include "definitions.v"'
    result = preprocess(source)
    assert_includes result, "// [preprocessor] include stubbed"
    assert_includes result, "definitions.v"
  end

  # ------------------------------------------------------------------
  # `timescale (stripped)
  # ------------------------------------------------------------------

  def test_timescale_stripped
    source = "`timescale 1ns/1ps\nmodule top;"
    result = preprocess(source)
    refute_includes result, "timescale"
    assert_includes result, "module top;"
  end

  # ------------------------------------------------------------------
  # Lines without directives pass through unchanged
  # ------------------------------------------------------------------

  def test_plain_lines_pass_through
    source = "wire a;\nassign a = 1;"
    result = preprocess(source)
    assert_equal "wire a;\nassign a = 1;", result
  end

  # ------------------------------------------------------------------
  # `define in inactive scope is ignored
  # ------------------------------------------------------------------

  def test_define_in_inactive_scope
    source = "`ifdef NOPE\n`define VAL 99\n`endif\nassign x = `VAL;"
    result = preprocess(source)
    # VAL should not be defined (was in an inactive ifdef)
    assert_includes result, "`VAL"
  end

  # ------------------------------------------------------------------
  # `undef in inactive scope is ignored
  # ------------------------------------------------------------------

  def test_undef_in_inactive_scope
    source = "`define VAL 42\n`ifdef NOPE\n`undef VAL\n`endif\nassign x = `VAL;"
    result = preprocess(source)
    assert_includes result, "42"
  end

  # ------------------------------------------------------------------
  # Edge case: empty source
  # ------------------------------------------------------------------

  def test_empty_source
    result = preprocess("")
    assert_equal "", result
  end

  # ------------------------------------------------------------------
  # Edge case: only directives, no output
  # ------------------------------------------------------------------

  def test_only_directives
    source = "`define A 1\n`define B 2"
    result = preprocess(source)
    assert_equal "", result
  end

  # ------------------------------------------------------------------
  # Include in inactive scope is suppressed
  # ------------------------------------------------------------------

  def test_include_in_inactive_scope
    source = "`ifdef NOPE\n`include \"file.v\"\n`endif"
    result = preprocess(source)
    refute_includes result, "include"
  end
end
