defmodule CodingAdventures.RubyParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.RubyParser

  defp find_nodes(%ASTNode{} = node, rule_name) do
    current = if node.rule_name == rule_name, do: [node], else: []

    Enum.reduce(node.children, current, fn
      %ASTNode{} = child, acc -> acc ++ find_nodes(child, rule_name)
      _child, acc -> acc
    end)
  end

  test "parses assignment statements" do
    {:ok, ast} = RubyParser.parse("x = 1 + 2")
    assert ast.rule_name == "program"
    assert length(find_nodes(ast, "assignment")) == 1
    assert length(find_nodes(ast, "expression")) >= 1
  end

  test "parses arithmetic precedence through term nesting" do
    {:ok, ast} = RubyParser.parse("x = 1 + 2 * 3")
    assert ast.rule_name == "program"
    assert length(find_nodes(ast, "term")) >= 2
  end

  test "parses method calls" do
    {:ok, ast} = RubyParser.parse("puts(\"hello\")")
    assert ast.rule_name == "program"
    assert length(find_nodes(ast, "method_call")) == 1
  end

  test "create_parser returns the parsed grammar" do
    grammar = RubyParser.create_parser()
    assert is_map(grammar)
    assert hd(grammar.rules).name == "program"
  end

  test "lexer errors are returned" do
    assert {:error, message} = RubyParser.parse("@")
    assert message =~ "Unexpected character"
  end

  test "malformed Ruby returns a parser error" do
    assert {:error, message} = RubyParser.parse("x = (1 + 2")
    assert message =~ "Expected"
  end
end
