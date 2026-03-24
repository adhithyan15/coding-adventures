defmodule CodingAdventures.Arm1SimulatorTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Arm1Simulator)
  end
end
