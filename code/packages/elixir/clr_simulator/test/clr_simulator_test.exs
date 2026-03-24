defmodule CodingAdventures.ClrSimulatorTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.ClrSimulator)
  end
end
