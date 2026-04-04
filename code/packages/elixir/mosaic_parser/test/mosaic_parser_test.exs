defmodule CodingAdventures.MosaicParserTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.MosaicParser)
  end
end
