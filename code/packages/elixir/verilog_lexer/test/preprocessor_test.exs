defmodule CodingAdventures.VerilogLexer.PreprocessorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.VerilogLexer.Preprocessor

  # ===========================================================================
  # `define / `undef — Simple Macros
  # ===========================================================================

  describe "process/1 — simple macros" do
    test "defines and expands a simple macro" do
      source = "`define WIDTH 8\nwire [`WIDTH-1:0] bus;"
      result = Preprocessor.process(source)
      assert result =~ "wire [8-1:0] bus;"
      # The `define line itself becomes empty
      refute result =~ "`define"
    end

    test "defines an empty (flag) macro" do
      source = "`define DEBUG\n`ifdef DEBUG\nwire dbg;\n`endif"
      result = Preprocessor.process(source)
      assert result =~ "wire dbg;"
    end

    test "undef removes a macro" do
      source = "`define WIDTH 8\n`undef WIDTH\nwire [`WIDTH:0] bus;"
      result = Preprocessor.process(source)
      # After `undef, `WIDTH is no longer expanded — it stays as-is
      assert result =~ "`WIDTH"
    end

    test "macro expansion in middle of line" do
      source = "`define DEPTH 16\nassign mem_size = `DEPTH * 2;"
      result = Preprocessor.process(source)
      assert result =~ "assign mem_size = 16 * 2;"
    end

    test "multiple macros on the same line" do
      source = "`define A 1\n`define B 2\nassign x = `A + `B;"
      result = Preprocessor.process(source)
      assert result =~ "assign x = 1 + 2;"
    end

    test "macro redefinition replaces previous value" do
      source = "`define VAL 10\nassign a = `VAL;\n`define VAL 20\nassign b = `VAL;"
      result = Preprocessor.process(source)
      assert result =~ "assign a = 10;"
      assert result =~ "assign b = 20;"
    end
  end

  # ===========================================================================
  # `define — Parameterized Macros
  # ===========================================================================

  describe "process/1 — parameterized macros" do
    test "defines and expands a parameterized macro" do
      source = "`define ADD(a, b) (a + b)\nassign sum = `ADD(x, y);"
      result = Preprocessor.process(source)
      assert result =~ "assign sum = (x + y);"
    end

    test "parameterized macro with nested parentheses in args" do
      source = "`define MAX(a, b) (a > b ? a : b)\nassign m = `MAX((x+1), y);"
      result = Preprocessor.process(source)
      assert result =~ "assign m = ((x+1) > y ? (x+1) : y);"
    end

    test "parameterized macro with single parameter" do
      source = "`define NEG(x) (~x + 1)\nassign n = `NEG(data);"
      result = Preprocessor.process(source)
      assert result =~ "assign n = (~data + 1);"
    end

    test "parameterized macro with three parameters" do
      source = "`define MUX(sel, a, b) (sel ? a : b)\nassign out = `MUX(s, d0, d1);"
      result = Preprocessor.process(source)
      assert result =~ "assign out = (s ? d0 : d1);"
    end
  end

  # ===========================================================================
  # parse_define/1 — Internal Parsing
  # ===========================================================================

  describe "parse_define/1" do
    test "parses simple macro" do
      {name, params, body} = Preprocessor.parse_define("WIDTH 8")
      assert name == "WIDTH"
      assert params == nil
      assert body == "8"
    end

    test "parses empty macro (flag)" do
      {name, params, body} = Preprocessor.parse_define("DEBUG")
      assert name == "DEBUG"
      assert params == nil
      assert body == ""
    end

    test "parses parameterized macro" do
      {name, params, body} = Preprocessor.parse_define("ADD(a, b) (a + b)")
      assert name == "ADD"
      assert params == ["a", "b"]
      assert body == "(a + b)"
    end

    test "parses parameterized macro with single param" do
      {name, params, body} = Preprocessor.parse_define("NEG(x) (~x)")
      assert name == "NEG"
      assert params == ["x"]
      assert body == "(~x)"
    end

    test "parses macro with complex body" do
      {name, params, body} = Preprocessor.parse_define("ADDR_WIDTH 32")
      assert name == "ADDR_WIDTH"
      assert params == nil
      assert body == "32"
    end
  end

  # ===========================================================================
  # `ifdef / `ifndef / `else / `endif — Conditional Compilation
  # ===========================================================================

  describe "process/1 — ifdef" do
    test "includes lines when macro is defined" do
      source = "`define DEBUG\n`ifdef DEBUG\nwire dbg;\n`endif"
      result = Preprocessor.process(source)
      assert result =~ "wire dbg;"
    end

    test "excludes lines when macro is not defined" do
      source = "`ifdef DEBUG\nwire dbg;\n`endif"
      result = Preprocessor.process(source)
      refute result =~ "wire dbg;"
    end

    test "ifdef with else — defined branch" do
      source = "`define FAST\n`ifdef FAST\nassign clk_div = 1;\n`else\nassign clk_div = 4;\n`endif"
      result = Preprocessor.process(source)
      assert result =~ "assign clk_div = 1;"
      refute result =~ "assign clk_div = 4;"
    end

    test "ifdef with else — undefined branch" do
      source = "`ifdef FAST\nassign clk_div = 1;\n`else\nassign clk_div = 4;\n`endif"
      result = Preprocessor.process(source)
      refute result =~ "assign clk_div = 1;"
      assert result =~ "assign clk_div = 4;"
    end
  end

  describe "process/1 — ifndef" do
    test "includes lines when macro is NOT defined" do
      source = "`ifndef GUARD\nwire data;\n`endif"
      result = Preprocessor.process(source)
      assert result =~ "wire data;"
    end

    test "excludes lines when macro IS defined" do
      source = "`define GUARD\n`ifndef GUARD\nwire data;\n`endif"
      result = Preprocessor.process(source)
      refute result =~ "wire data;"
    end

    test "ifndef with else" do
      source = "`ifndef SYNTHESIS\n// sim only\n`else\n// synth only\n`endif"
      result = Preprocessor.process(source)
      assert result =~ "// sim only"
      refute result =~ "// synth only"
    end
  end

  describe "process/1 — nested conditionals" do
    test "nested ifdef blocks" do
      source = """
      `define A
      `define B
      `ifdef A
      wire a;
      `ifdef B
      wire ab;
      `endif
      `endif
      """

      result = Preprocessor.process(source)
      assert result =~ "wire a;"
      assert result =~ "wire ab;"
    end

    test "nested ifdef — inner false" do
      source = """
      `define A
      `ifdef A
      wire a;
      `ifdef B
      wire ab;
      `endif
      `endif
      """

      result = Preprocessor.process(source)
      assert result =~ "wire a;"
      refute result =~ "wire ab;"
    end

    test "nested ifdef — outer false" do
      source = """
      `define B
      `ifdef A
      wire a;
      `ifdef B
      wire ab;
      `endif
      `endif
      """

      result = Preprocessor.process(source)
      refute result =~ "wire a;"
      refute result =~ "wire ab;"
    end
  end

  # ===========================================================================
  # `include — File Inclusion (Stubbed)
  # ===========================================================================

  describe "process/1 — include" do
    test "replaces include with a comment placeholder" do
      source = ~s(`include "definitions.vh"\nmodule top; endmodule)
      result = Preprocessor.process(source)
      assert result =~ "// [include: definitions.vh]"
      assert result =~ "module top; endmodule"
    end

    test "handles include inside inactive block" do
      source = ~s(`ifdef MISSING\n`include "other.v"\n`endif)
      result = Preprocessor.process(source)
      refute result =~ "// [include: other.v]"
    end
  end

  # ===========================================================================
  # `timescale — Stripped
  # ===========================================================================

  describe "process/1 — timescale" do
    test "strips timescale directive" do
      source = "`timescale 1ns/1ps\nmodule top; endmodule"
      result = Preprocessor.process(source)
      refute result =~ "timescale"
      assert result =~ "module top; endmodule"
    end

    test "strips timescale even with different units" do
      source = "`timescale 10us/100ns"
      result = Preprocessor.process(source)
      refute result =~ "timescale"
    end
  end

  # ===========================================================================
  # expand_macros/2 — Edge Cases
  # ===========================================================================

  describe "expand_macros/2" do
    test "returns line unchanged when no macros defined" do
      result = Preprocessor.expand_macros("wire [7:0] data;", %{})
      assert result == "wire [7:0] data;"
    end

    test "does not expand undefined macro references" do
      macros = %{"WIDTH" => {nil, "8"}}
      result = Preprocessor.expand_macros("assign x = `DEPTH;", macros)
      # `DEPTH is not defined, so it stays as-is
      assert result =~ "`DEPTH"
    end

    test "handles multiple expansions on one line" do
      macros = %{"A" => {nil, "1"}, "B" => {nil, "2"}}
      result = Preprocessor.expand_macros("`A + `B", macros)
      assert result == "1 + 2"
    end
  end

  # ===========================================================================
  # Full Pipeline — Preprocessing + Tokenization
  # ===========================================================================

  describe "full pipeline — preprocess then tokenize" do
    test "preprocesses and tokenizes a complete Verilog snippet" do
      source = """
      `timescale 1ns/1ps
      `define WIDTH 8

      module counter(
        input clk,
        output reg [`WIDTH-1:0] count
      );
        always @(posedge clk)
          count <= count + 1;
      endmodule
      """

      {:ok, tokens} = CodingAdventures.VerilogLexer.tokenize(source, preprocess: true)

      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      # Should have module structure tokens
      assert "KEYWORD" in types
      assert "NAME" in types
      assert "LPAREN" in types
      assert "RPAREN" in types
      assert "SEMICOLON" in types

      # The WIDTH macro should have been expanded to 8
      values =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.value)

      assert "8" in values
      assert "counter" in values
      assert "clk" in values
    end
  end

  # ===========================================================================
  # Edge Cases
  # ===========================================================================

  describe "process/1 — edge cases" do
    test "empty source" do
      result = Preprocessor.process("")
      assert result == ""
    end

    test "source with no directives" do
      source = "module top; endmodule"
      result = Preprocessor.process(source)
      assert result == "module top; endmodule"
    end

    test "else without ifdef is handled gracefully" do
      source = "`else\nwire x;\n`endif"
      # Should not crash — graceful handling
      result = Preprocessor.process(source)
      assert is_binary(result)
    end

    test "endif without ifdef is handled gracefully" do
      source = "`endif\nwire x;"
      result = Preprocessor.process(source)
      assert is_binary(result)
    end

    test "define with leading whitespace on code line" do
      source = "`define VAL 42\n  assign x = `VAL;"
      result = Preprocessor.process(source)
      assert result =~ "assign x = 42;"
    end
  end
end
