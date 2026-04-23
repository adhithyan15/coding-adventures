defmodule CodingAdventures.HaskellParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HaskellParser

  test "parser root is file" do
    {:ok, ast} = HaskellParser.parse("x")
    assert ast.rule_name == "file"
  end

  test "explicit-brace let parses under 2010" do
    {:ok, ast} = HaskellParser.parse("let { x = y } in x", "2010")
    assert ast.rule_name == "file"
  end
end
