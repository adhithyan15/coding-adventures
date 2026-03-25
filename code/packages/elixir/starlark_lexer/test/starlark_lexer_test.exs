defmodule CodingAdventures.StarlarkLexerTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.StarlarkLexer)
  end
end
