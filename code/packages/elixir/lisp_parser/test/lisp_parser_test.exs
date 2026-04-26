defmodule CodingAdventures.LispParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.LispParser

  test "parses a basic definition" do
    {:ok, ast} = LispParser.parse("(define x 42)")
    assert ast.rule_name == "program"
    assert ast.children != []
  end

  test "parses quoted forms" do
    {:ok, ast} = LispParser.parse("'(a b c)")
    assert ast.rule_name == "program"
  end

  test "parses dotted pairs" do
    {:ok, ast} = LispParser.parse("(a . b)")
    assert ast.rule_name == "program"
  end

  test "create_parser returns the parsed grammar" do
    grammar = LispParser.create_parser()
    assert is_map(grammar)
    assert hd(grammar.rules).name == "program"
  end

  test "invalid lexer input is returned as an error" do
    assert {:error, message} = LispParser.parse("@")
    assert message =~ "Unexpected character"
  end

  test "malformed lists return a parser error" do
    assert {:error, message} = LispParser.parse("(a b")
    assert message =~ "Expected"
  end
end
