defmodule CodingAdventures.CacheTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Cache)
  end
end
