defmodule CodingAdventures.EcmascriptEs3ParserTest do
  use ExUnit.Case

  alias CodingAdventures.EcmascriptEs3Parser
  alias CodingAdventures.Parser.ASTNode

  # ===========================================================================
  # Module loading
  # ===========================================================================

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.EcmascriptEs3Parser)
  end

  # ===========================================================================
  # Grammar loading
  # ===========================================================================

  test "grammar_path returns a path ending in es3.grammar" do
    path = EcmascriptEs3Parser.grammar_path()
    assert String.ends_with?(path, "es3.grammar")
  end

  test "load_grammar succeeds" do
    assert {:ok, grammar} = EcmascriptEs3Parser.load_grammar()
    assert length(grammar.rules) > 0
  end

  # ===========================================================================
  # Basic parsing (inherited from ES1)
  # ===========================================================================

  test "parse empty program" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("")
    assert %ASTNode{rule_name: "program"} = ast
  end

  test "parse var declaration" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("var x = 1;")
    assert ast.rule_name == "program"
  end

  test "parse function declaration" do
    source = "function add(a, b) { return a + b; }"
    assert {:ok, ast} = EcmascriptEs3Parser.parse(source)
    assert ast.rule_name == "program"
  end

  test "parse if-else statement" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("if (x) { y; } else { z; }")
    assert ast.rule_name == "program"
  end

  test "parse while loop" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("while (x) { y; }")
    assert ast.rule_name == "program"
  end

  test "parse for loop" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("for (var i = 0; i; i) { x; }")
    assert ast.rule_name == "program"
  end

  # ===========================================================================
  # ES3-specific: try/catch/finally/throw
  # ===========================================================================

  test "parse try/catch" do
    source = "try { x(); } catch (e) { y(); }"
    assert {:ok, ast} = EcmascriptEs3Parser.parse(source)
    assert ast.rule_name == "program"
  end

  test "parse try/finally" do
    source = "try { x(); } finally { cleanup(); }"
    assert {:ok, ast} = EcmascriptEs3Parser.parse(source)
    assert ast.rule_name == "program"
  end

  test "parse try/catch/finally" do
    source = "try { x(); } catch (e) { y(); } finally { z(); }"
    assert {:ok, ast} = EcmascriptEs3Parser.parse(source)
    assert ast.rule_name == "program"
  end

  test "parse throw statement" do
    source = "throw new Error();"
    assert {:ok, ast} = EcmascriptEs3Parser.parse(source)
    assert ast.rule_name == "program"
  end

  # ===========================================================================
  # ES3-specific: strict equality
  # ===========================================================================

  test "parse strict equality" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("x === 1;")
    assert ast.rule_name == "program"
  end

  test "parse strict not-equals" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("x !== 1;")
    assert ast.rule_name == "program"
  end

  # ===========================================================================
  # ES3-specific: instanceof
  # ===========================================================================

  test "parse instanceof expression" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("x instanceof Array;")
    assert ast.rule_name == "program"
  end

  # ===========================================================================
  # Expressions
  # ===========================================================================

  test "parse arithmetic expression" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("1 + 2 * 3;")
    assert ast.rule_name == "program"
  end

  test "parse ternary expression" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("x ? 1 : 2;")
    assert ast.rule_name == "program"
  end

  test "parse object literal" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("var o = { a: 1, b: 2 };")
    assert ast.rule_name == "program"
  end

  test "parse array literal" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("var a = [1, 2, 3];")
    assert ast.rule_name == "program"
  end

  test "parse function call" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("foo(1, 2);")
    assert ast.rule_name == "program"
  end

  test "parse new expression" do
    assert {:ok, ast} = EcmascriptEs3Parser.parse("new Foo(1);")
    assert ast.rule_name == "program"
  end
end
