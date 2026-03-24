defmodule CodingAdventures.JvmSimulatorTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.JvmSimulator)
  end
end
