defmodule CodingAdventures.TypescriptParserTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.TypescriptParser)
  end
end
