defmodule CodingAdventures.VerilogParserTest do
  use ExUnit.Case

  alias CodingAdventures.VerilogParser

  test "parses a basic Verilog module" do
    source = "module test (a, b);
  input a;
  output b;
  assign b = a;
endmodule
"
    assert {:ok, _ast} = VerilogParser.parse(source)
  end

  test "fails on invalid syntax" do
    assert {:error, _} = VerilogParser.parse("this is not verilog")
  end
end
