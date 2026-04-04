defmodule CodingAdventures.MosaicVmTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.MosaicVm)
  end
end
