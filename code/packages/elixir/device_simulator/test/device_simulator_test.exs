defmodule CodingAdventures.DeviceSimulatorTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.DeviceSimulator)
  end
end
