defmodule CodingAdventures.PythonParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.PythonParser

  defp find_nodes(%ASTNode{} = node, rule_name) do
    current = if node.rule_name == rule_name, do: [node], else: []

    Enum.reduce(node.children, current, fn
      %ASTNode{} = child, acc -> acc ++ find_nodes(child, rule_name)
      _child, acc -> acc
    end)
  end

  test "defaults to Python 3.12 and lists every supported grammar version" do
    assert PythonParser.default_version() == "3.12"

    assert PythonParser.supported_versions() == [
             "2.7",
             "3.0",
             "3.6",
             "3.8",
             "3.10",
             "3.12"
           ]
  end

  test "parses assignment statements with the default grammar" do
    {:ok, ast} = PythonParser.parse("x = 1 + 2\n")
    assert ast.rule_name == "program"
    assert length(find_nodes(ast, "assignment")) == 1
    assert length(find_nodes(ast, "expression")) >= 1
  end

  test "preserves arithmetic precedence through term nesting" do
    {:ok, ast} = PythonParser.parse("x = 1 + 2 * 3\n")
    assert ast.rule_name == "program"
    assert length(find_nodes(ast, "term")) >= 2
  end

  test "parses source with an explicit historical lexer grammar" do
    {:ok, ast} = PythonParser.parse("x = 1\n", "2.7")
    assert ast.rule_name == "program"
    assert length(find_nodes(ast, "assignment")) == 1
  end

  test "nil and empty versions select the default grammar" do
    assert {:ok, %ASTNode{rule_name: "program"}} = PythonParser.parse("x = 1\n", nil)
    assert {:ok, %ASTNode{rule_name: "program"}} = PythonParser.parse("x = 1\n", "")
  end

  test "create_parser returns and caches the shared parsed grammar" do
    grammar = PythonParser.create_parser()
    assert is_map(grammar)
    assert hd(grammar.rules).name == "program"
    assert PythonParser.create_parser() === grammar
  end

  test "unknown versions raise a useful error" do
    assert_raise ArgumentError, ~r/Unknown Python version "3.11"/, fn ->
      PythonParser.parse("x = 1\n", "3.11")
    end
  end

  test "lexer errors are returned" do
    assert {:error, message} = PythonParser.parse("x = \u00A7\n")
    assert message =~ "Unexpected character"
  end

  test "malformed Python returns a parser error" do
    assert {:error, message} = PythonParser.parse("x = (1 + 2\n")
    assert message =~ "Expected"
  end
end
