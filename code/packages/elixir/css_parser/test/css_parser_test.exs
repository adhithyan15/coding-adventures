defmodule CodingAdventures.CssParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CssParser
  alias CodingAdventures.Parser.ASTNode

  defp find_nodes(%ASTNode{} = node, rule_name) do
    current = if node.rule_name == rule_name, do: [node], else: []

    Enum.reduce(node.children, current, fn
      %ASTNode{} = child, acc -> acc ++ find_nodes(child, rule_name)
      _child, acc -> acc
    end)
  end

  test "parses an empty stylesheet" do
    {:ok, ast} = CssParser.parse("")
    assert ast.rule_name == "stylesheet"
    assert ast.children == []
  end

  test "parses a qualified rule with a declaration" do
    {:ok, ast} = CssParser.parse("h1 { color: red; }")
    assert ast.rule_name == "stylesheet"
    assert length(find_nodes(ast, "rule")) == 1
    assert length(find_nodes(ast, "declaration")) == 1
  end

  test "parses selector lists" do
    {:ok, ast} = CssParser.parse("h1, h2, h3 { font-weight: bold; }")
    assert length(find_nodes(ast, "complex_selector")) == 3
  end

  test "parses at-rules" do
    {:ok, ast} = CssParser.parse(~s(@import "theme.css";))
    assert length(find_nodes(ast, "at_rule")) == 1
  end

  test "create_parser returns the parsed grammar" do
    grammar = CssParser.create_parser()
    assert is_map(grammar)
    assert hd(grammar.rules).name == "stylesheet"
  end

  test "lexer errors are returned" do
    assert {:error, message} = CssParser.parse("`")
    assert message =~ "Unexpected character"
  end

  test "malformed CSS returns a parser error" do
    assert {:error, message} = CssParser.parse("h1 { color: red")
    assert message =~ "Expected"
  end
end
