defmodule CodingAdventures.VerilogParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.VerilogParser

  describe "create_parser/0" do
    test "returns a ParserGrammar with the expected entry rule" do
      grammar = VerilogParser.create_parser()
      assert hd(grammar.rules).name == "source_text"
    end

    test "supports selecting an explicit language edition" do
      default_rule_names = Enum.map(VerilogParser.create_parser().rules, & &1.name)
      versioned_rule_names = Enum.map(VerilogParser.create_parser("2005").rules, & &1.name)

      assert default_rule_names == versioned_rule_names
    end

    test "raises for an unknown language edition" do
      assert_raise ArgumentError, ~r/Unknown Verilog version/, fn ->
        VerilogParser.create_parser("2099")
      end
    end
  end

  describe "parse/2" do
    test "parses a simple module" do
      {:ok, %ASTNode{} = ast} = VerilogParser.parse("module empty; endmodule")
      assert ast.rule_name == "source_text"
    end

    test "supports preprocessing and explicit versions together" do
      source = "`define WIDTH 8\nmodule sized; wire [`WIDTH-1:0] bus; endmodule"

      {:ok, %ASTNode{} = ast} =
        VerilogParser.parse(source, preprocess: true, version: "2005")

      assert ast.rule_name == "source_text"
    end
  end
end
