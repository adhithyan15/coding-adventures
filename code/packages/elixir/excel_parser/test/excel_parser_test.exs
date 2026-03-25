defmodule CodingAdventures.ExcelParserTest do
  use ExUnit.Case

  test "parses function formulas" do
    {:ok, ast} = CodingAdventures.ExcelParser.parse("=SUM(A1:B2)")
    assert ast.rule_name == "formula"
  end

  test "parses column ranges" do
    {:ok, ast} = CodingAdventures.ExcelParser.parse("A:C")
    assert ast.rule_name == "formula"
  end

  test "parses row ranges" do
    {:ok, ast} = CodingAdventures.ExcelParser.parse("1:3")
    assert ast.rule_name == "formula"
  end
end
