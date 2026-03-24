defmodule CodingAdventures.JavascriptLexerTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.JavascriptLexer)
  end
end
