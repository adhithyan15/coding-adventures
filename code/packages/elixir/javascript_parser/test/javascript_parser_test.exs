defmodule CodingAdventures.JavascriptParserTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.JavascriptParser)
  end
end
