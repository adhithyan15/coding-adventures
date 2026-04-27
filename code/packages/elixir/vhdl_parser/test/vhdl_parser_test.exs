defmodule CodingAdventures.VhdlParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.VhdlParser

  describe "create_parser/0" do
    test "returns a ParserGrammar with the expected entry rule" do
      grammar = VhdlParser.create_parser()
      assert hd(grammar.rules).name == "design_file"
    end

    test "reports supported versions and default version" do
      assert VhdlParser.default_version() == "2008"
      assert VhdlParser.supported_versions() == ~w(1987 1993 2002 2008 2019)
      assert VhdlParser.resolve_version!(nil) == "2008"
      assert VhdlParser.resolve_version!("") == "2008"
    end

    test "supports selecting every explicit language edition" do
      default_rule_names = Enum.map(VhdlParser.create_parser().rules, & &1.name)

      for version <- VhdlParser.supported_versions() do
        versioned_rule_names = Enum.map(VhdlParser.create_parser(version).rules, & &1.name)
        assert versioned_rule_names == default_rule_names
        assert VhdlParser.resolve_version!(version) == version
      end
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

    test "supports every explicit version" do
      for version <- VhdlParser.supported_versions() do
        {:ok, %ASTNode{} = ast} =
          VhdlParser.parse("entity empty is end entity empty;", version: version)

        assert ast.rule_name == "design_file"
      end
    end
  end
end
