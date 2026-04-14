defmodule CodingAdventures.VhdlParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.VhdlParser

  describe "create_parser/0" do
    test "returns a ParserGrammar with the expected entry rule" do
      grammar = VhdlParser.create_parser()
      assert hd(grammar.rules).name == "design_file"
    end

    test "supports selecting an explicit language edition" do
      default_rule_names = Enum.map(VhdlParser.create_parser().rules, & &1.name)
      versioned_rule_names = Enum.map(VhdlParser.create_parser("2008").rules, & &1.name)

      assert default_rule_names == versioned_rule_names
    end

    test "raises for an unknown language edition" do
      assert_raise ArgumentError, ~r/Unknown VHDL version/, fn ->
        VhdlParser.create_parser("2099")
      end
    end
  end

  describe "parse/2" do
    test "parses a simple entity and architecture" do
      source = """
      entity empty is
      end entity empty;

      architecture rtl of empty is
      begin
      end architecture rtl;
      """

      {:ok, %ASTNode{} = ast} = VhdlParser.parse(source)
      assert ast.rule_name == "design_file"
    end

    test "supports explicit versions" do
      {:ok, %ASTNode{} = ast} =
        VhdlParser.parse("entity empty is end entity empty;", version: "2008")

      assert ast.rule_name == "design_file"
    end
  end
end
