defmodule CodingAdventures.ArmSimulatorTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.ArmSimulator)
  end
end
