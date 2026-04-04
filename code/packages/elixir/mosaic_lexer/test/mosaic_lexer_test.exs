defmodule CodingAdventures.MosaicLexerTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.MosaicLexer)
  end
end
