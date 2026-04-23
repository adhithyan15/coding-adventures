defmodule CodingAdventures.HaskellParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HaskellParser

  test "default_version exposes 2010" do
    assert HaskellParser.default_version() == "2010"
  end

  test "supported_versions includes all versioned grammars" do
    assert HaskellParser.supported_versions() == ~w(1.0 1.1 1.2 1.3 1.4 98 2010)
  end

  test "parser root is file" do
    {:ok, ast} = HaskellParser.parse("x")
    assert ast.rule_name == "file"
  end

  test "empty string version falls back to the default grammar" do
    {:ok, ast} = HaskellParser.parse("x", "")
    assert ast.rule_name == "file"
  end

  test "explicit-brace let parses under 2010" do
    {:ok, ast} = HaskellParser.parse("let { x = y } in x", "2010")
    assert ast.rule_name == "file"
  end

  test "historical versions are routable" do
    {:ok, ast} = HaskellParser.parse("x", "98")
    assert ast.rule_name == "file"
  end

  test "invalid lexer input is returned as an error" do
    assert {:error, message} = HaskellParser.parse("@")
    assert message =~ "Unexpected character"
  end

  test "create_parser returns a parser grammar for a historical version" do
    grammar = HaskellParser.create_parser("98")

    assert is_map(grammar)
    assert hd(grammar.rules).name == "file"
  end

  test "unknown string versions raise a helpful error" do
    assert_raise ArgumentError, ~r/Unknown Haskell version/, fn ->
      HaskellParser.create_parser("99")
    end
  end

  test "non-binary versions raise a helpful error" do
    assert_raise ArgumentError, ~r/Unknown Haskell version/, fn ->
      HaskellParser.create_parser(98)
    end
  end
end
