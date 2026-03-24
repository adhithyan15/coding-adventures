defmodule CodingAdventures.WasmSimulatorTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.WasmSimulator)
  end
end
