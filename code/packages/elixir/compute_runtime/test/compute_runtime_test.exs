defmodule CodingAdventures.ComputeRuntimeTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.ComputeRuntime)
  end
end
