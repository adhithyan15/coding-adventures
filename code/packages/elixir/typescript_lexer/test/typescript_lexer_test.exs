defmodule CodingAdventures.TypescriptLexerTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.TypescriptLexer)
  end
end
