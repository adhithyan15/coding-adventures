defmodule CodingAdventures.UuidTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Uuid)
  end
end
