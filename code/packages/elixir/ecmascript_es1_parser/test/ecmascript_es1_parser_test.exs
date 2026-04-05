defmodule CodingAdventures.EcmascriptEs1ParserTest do
  use ExUnit.Case

  alias CodingAdventures.EcmascriptEs1Parser
  alias CodingAdventures.Parser.ASTNode

  # ===========================================================================
  # Module loading
  # ===========================================================================

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.EcmascriptEs1Parser)
  end

  # ===========================================================================
  # Grammar loading
  # ===========================================================================

  test "grammar_path returns a path ending in es1.grammar" do
    path = EcmascriptEs1Parser.grammar_path()
    assert String.ends_with?(path, "es1.grammar")
  end

  test "load_grammar succeeds" do
    assert {:ok, grammar} = EcmascriptEs1Parser.load_grammar()
    assert length(grammar.rules) > 0
  end

  # ===========================================================================
  # Basic parsing
  # ===========================================================================

  test "parse empty program" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("")
    assert %ASTNode{rule_name: "program"} = ast
  end

  test "parse var declaration" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("var x = 1;")
    assert ast.rule_name == "program"
    assert length(ast.children) > 0
  end

  test "parse multiple var declarations" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("var x = 1, y = 2;")
    assert ast.rule_name == "program"
  end

  test "parse expression statement" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("1 + 2;")
    assert ast.rule_name == "program"
  end

  test "parse string literal" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse(~s(var s = "hello";))
    assert ast.rule_name == "program"
  end

  # ===========================================================================
  # Statements
  # ===========================================================================

  test "parse if statement" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("if (x) { y; }")
    assert ast.rule_name == "program"
  end

  test "parse if-else statement" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("if (x) { y; } else { z; }")
    assert ast.rule_name == "program"
  end

  test "parse while loop" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("while (x) { y; }")
    assert ast.rule_name == "program"
  end

  test "parse do-while loop" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("do { x; } while (y);")
    assert ast.rule_name == "program"
  end

  test "parse for loop" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("for (var i = 0; i; i) { x; }")
    assert ast.rule_name == "program"
  end

  test "parse empty statement" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse(";")
    assert ast.rule_name == "program"
  end

  test "parse block statement" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("{ x; y; }")
    assert ast.rule_name == "program"
  end

  test "parse return statement" do
    source = "function f() { return 1; }"
    assert {:ok, ast} = EcmascriptEs1Parser.parse(source)
    assert ast.rule_name == "program"
  end

  test "parse switch statement" do
    source = "switch (x) { case 1: break; default: y; }"
    assert {:ok, ast} = EcmascriptEs1Parser.parse(source)
    assert ast.rule_name == "program"
  end

  # ===========================================================================
  # Functions
  # ===========================================================================

  test "parse function declaration" do
    source = "function add(a, b) { return a + b; }"
    assert {:ok, ast} = EcmascriptEs1Parser.parse(source)
    assert ast.rule_name == "program"
  end

  test "parse function expression" do
    source = "var f = function(x) { return x; };"
    assert {:ok, ast} = EcmascriptEs1Parser.parse(source)
    assert ast.rule_name == "program"
  end

  # ===========================================================================
  # Expressions
  # ===========================================================================

  test "parse arithmetic expression" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("1 + 2 * 3;")
    assert ast.rule_name == "program"
  end

  test "parse comparison expression" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("a == b;")
    assert ast.rule_name == "program"
  end

  test "parse ternary expression" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("x ? 1 : 2;")
    assert ast.rule_name == "program"
  end

  test "parse assignment via var" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("var x = 1;")
    assert ast.rule_name == "program"
  end

  test "parse object literal" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("var o = { a: 1, b: 2 };")
    assert ast.rule_name == "program"
  end

  test "parse array literal" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("var a = [1, 2, 3];")
    assert ast.rule_name == "program"
  end

  test "parse function call" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("foo(1, 2);")
    assert ast.rule_name == "program"
  end

  test "parse member access" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("a.b.c;")
    assert ast.rule_name == "program"
  end

  test "parse computed member access" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("a[0];")
    assert ast.rule_name == "program"
  end

  test "parse new expression" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("new Foo(1);")
    assert ast.rule_name == "program"
  end

  test "parse unary expression" do
    assert {:ok, ast} = EcmascriptEs1Parser.parse("typeof x;")
    assert ast.rule_name == "program"
  end
end
