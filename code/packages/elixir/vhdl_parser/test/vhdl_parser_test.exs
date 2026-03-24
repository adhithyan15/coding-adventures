defmodule CodingAdventures.VhdlParserTest do
  use ExUnit.Case

  alias CodingAdventures.VhdlParser

  test "parses a basic VHDL entity" do
    source = "entity test is
  port(
    a : in bit;
    b : out bit
  );
end test;
"
    assert {:ok, _ast} = VhdlParser.parse(source)
  end

  test "fails on invalid syntax" do
    assert {:error, _} = VhdlParser.parse("this is not vhdl")
  end
end
