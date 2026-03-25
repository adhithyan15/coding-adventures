defmodule CodingAdventures.ExcelLexerTest do
  use ExUnit.Case

  test "reclassifies function names" do
    {:ok, tokens} = CodingAdventures.ExcelLexer.tokenize("=SUM(A1)")
    assert Enum.at(tokens, 1).type == "FUNCTION_NAME"
  end

  test "reclassifies table names" do
    {:ok, tokens} = CodingAdventures.ExcelLexer.tokenize("DeptSales[Sales Amount]")
    assert hd(tokens).type == "TABLE_NAME"
  end
end
