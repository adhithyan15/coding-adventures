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

  describe "statement rule regression coverage (post grammar recompilation)" do
    # Regression test: ensure def/class/if/while/case/begin statement rules
    # are present after grammar recompilation. A stale compiled grammar was
    # previously found in the Ruby and TypeScript ports of this same package
    # (compiled from an old copy of `ruby.grammar` that predated later
    # extensions) silently missing dozens of statement rules, including all
    # six exercised here. This package now compiles its grammar fresh from
    # `code/grammars/ruby/ruby.grammar` via `grammar-tools compile-grammar`,
    # so these constructs must parse and produce the expected rule nodes.

    test "parses def statements" do
      {:ok, ast} = RubyParser.parse("def foo\n  x = 1\nend")
      assert ast.rule_name == "program"
      assert length(find_nodes(ast, "def_statement")) == 1
    end

    test "parses class statements" do
      {:ok, ast} = RubyParser.parse("class Foo\n  x = 1\nend")
      assert ast.rule_name == "program"
      assert length(find_nodes(ast, "class_statement")) == 1
    end

    test "parses if statements" do
      {:ok, ast} = RubyParser.parse("if x then y end")
      assert ast.rule_name == "program"
      assert length(find_nodes(ast, "if_statement")) == 1
    end

    test "parses if statements without then" do
      {:ok, ast} = RubyParser.parse("if x\n  y\nend")
      assert ast.rule_name == "program"
      assert length(find_nodes(ast, "if_statement")) == 1
    end

    test "parses while statements" do
      {:ok, ast} = RubyParser.parse("while x do y end")
      assert ast.rule_name == "program"
      assert length(find_nodes(ast, "while_statement")) == 1
    end

    test "parses case statements" do
      {:ok, ast} = RubyParser.parse("case x when 1 then y end")
      assert ast.rule_name == "program"
      assert length(find_nodes(ast, "case_statement")) == 1
    end

    test "parses begin/rescue statements" do
      {:ok, ast} = RubyParser.parse("begin x rescue y end")
      assert ast.rule_name == "program"
      assert length(find_nodes(ast, "begin_statement")) == 1
    end
  end
end
