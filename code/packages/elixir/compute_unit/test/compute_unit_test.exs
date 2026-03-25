defmodule CodingAdventures.ComputeUnitTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.ComputeUnit)
  end
end
