defmodule CodingAdventures.NibParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.NibParser

  test "parses a simple program" do
    {:ok, ast} = NibParser.parse_nib("fn main() { return 0; }")
    assert ast.rule_name == "program"
    assert length(ast.children) > 0
  end

  test "parses a loop" do
    {:ok, ast} = NibParser.parse_nib("fn main() { for i: u4 in 0..4 { return i; } }")
    assert ast.rule_name == "program"
  end
end
