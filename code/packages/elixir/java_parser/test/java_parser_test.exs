defmodule CodingAdventures.JavaParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.JavaParser
  alias CodingAdventures.Parser.ASTNode

  describe "version helpers" do
    test "default_version is Java 21" do
      assert JavaParser.default_version() == "21"
    end

    test "supported_versions includes all expected releases" do
      versions = JavaParser.supported_versions()

      assert "1.0" in versions
      assert "1.1" in versions
      assert "1.4" in versions
      assert "5" in versions
      assert "7" in versions
      assert "8" in versions
      assert "10" in versions
      assert "14" in versions
      assert "17" in versions
      assert "21" in versions
    end
  end

  describe "parse/2" do
    test "parses a simple class declaration with the default grammar" do
      assert {:ok, ast} = JavaParser.parse("public class Hello { }")
      assert ast.rule_name == "program"
      assert length(ASTNode.find_nodes(ast, "class_declaration")) == 1
    end

    test "treats nil and empty version as Java 21" do
      assert {:ok, nil_version_ast} = JavaParser.parse("class Hello { }", nil)
      assert {:ok, empty_version_ast} = JavaParser.parse("class Hello { }", "")

      assert nil_version_ast.rule_name == empty_version_ast.rule_name
      assert length(ASTNode.find_nodes(nil_version_ast, "class_declaration")) == 1
    end

    test "accepts all declared version strings" do
      for version <- JavaParser.supported_versions() do
        assert {:ok, ast} = JavaParser.parse("class Hello { }", version)
        assert ast.rule_name == "program"
      end
    end

    test "parses the empty compilation unit" do
      assert {:ok, ast} = JavaParser.parse("")
      assert ast.rule_name == "program"
      assert ast.children == []
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown Java version "99"/, fn ->
        JavaParser.parse("class Hello { }", "99")
      end
    end
  end

  describe "create_parser/1" do
    test "returns the parsed parser grammar for the default version" do
      grammar = JavaParser.create_parser()

      assert is_map(grammar)
      assert grammar.version == 1
      assert hd(grammar.rules).name == "program"
    end

    test "returns the parsed parser grammar for a specific version" do
      grammar = JavaParser.create_parser("8")

      assert is_map(grammar)
      assert grammar.version == 1
      assert hd(grammar.rules).name == "program"
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown Java version/, fn ->
        JavaParser.create_parser("latest")
      end
    end
  end
end
