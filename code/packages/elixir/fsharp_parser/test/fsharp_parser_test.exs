defmodule CodingAdventures.FSharpParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FSharpParser
  alias CodingAdventures.Parser.ASTNode

  describe "version helpers" do
    test "default_version is F# 10" do
      assert FSharpParser.default_version() == "10"
    end

    test "supported_versions includes all expected releases" do
      versions = FSharpParser.supported_versions()

      assert "1.0" in versions
      assert "2.0" in versions
      assert "3.0" in versions
      assert "3.1" in versions
      assert "4.0" in versions
      assert "4.1" in versions
      assert "4.5" in versions
      assert "4.6" in versions
      assert "4.7" in versions
      assert "5" in versions
      assert "6" in versions
      assert "7" in versions
      assert "8" in versions
      assert "9" in versions
      assert "10" in versions
    end
  end

  describe "parse/2" do
    test "parses a simple let binding with the default grammar" do
      assert {:ok, ast} = FSharpParser.parse("let value = 1")
      assert ast.rule_name == "compilation_unit"
      assert length(ASTNode.find_nodes(ast, "let_binding")) == 1
    end

    test "parses a small entry point with attributes and layout-sensitive newlines" do
      source = """
      [<EntryPoint>]
      let main _ =
          printfn "Hello, World!"
          0
      """

      assert {:ok, ast} = FSharpParser.parse(source)
      assert ast.rule_name == "compilation_unit"
      assert length(ASTNode.find_nodes(ast, "attribute_section")) == 1
      assert length(ASTNode.find_nodes(ast, "let_binding")) == 1
    end

    test "treats nil and empty version as F# 10" do
      assert {:ok, nil_version_ast} = FSharpParser.parse("let value = 1", nil)
      assert {:ok, empty_version_ast} = FSharpParser.parse("let value = 1", "")

      assert nil_version_ast.rule_name == empty_version_ast.rule_name
      assert length(ASTNode.find_nodes(nil_version_ast, "let_binding")) == 1
    end

    test "accepts all declared version strings" do
      for version <- FSharpParser.supported_versions() do
        assert {:ok, ast} = FSharpParser.parse("let value = 1", version)
        assert ast.rule_name == "compilation_unit"
      end
    end

    test "parses the empty compilation unit" do
      assert {:ok, ast} = FSharpParser.parse("")
      assert ast.rule_name == "compilation_unit"
      assert ast.children == []
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown F# version "99"/, fn ->
        FSharpParser.parse("let value = 1", "99")
      end
    end
  end

  describe "create_parser/1" do
    test "returns the parsed parser grammar for the default version" do
      grammar = FSharpParser.create_parser()

      assert is_map(grammar)
      assert grammar.version == 1
      assert hd(grammar.rules).name == "compilation_unit"
    end

    test "returns the parsed parser grammar for a specific version" do
      grammar = FSharpParser.create_parser("4.0")

      assert is_map(grammar)
      assert grammar.version == 1
      assert hd(grammar.rules).name == "compilation_unit"
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown F# version/, fn ->
        FSharpParser.create_parser("latest")
      end
    end
  end
end
