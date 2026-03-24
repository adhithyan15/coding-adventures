defmodule CodingAdventures.VendorApiSimulatorsTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.VendorApiSimulators)
  end
end
