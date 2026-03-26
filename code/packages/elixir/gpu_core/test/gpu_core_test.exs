defmodule CodingAdventures.GpuCoreTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.GpuCore)
  end
end
