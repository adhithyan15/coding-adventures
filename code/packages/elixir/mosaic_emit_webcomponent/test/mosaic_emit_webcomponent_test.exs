defmodule CodingAdventures.MosaicEmitWebcomponentTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.MosaicEmitWebcomponent)
  end
end
