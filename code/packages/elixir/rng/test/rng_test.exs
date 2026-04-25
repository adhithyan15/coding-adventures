defmodule CodingAdventures.RngTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Rng)
  end
end
