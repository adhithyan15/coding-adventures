defmodule CodingAdventures.StarlarkParserTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.StarlarkParser)
  end
end
