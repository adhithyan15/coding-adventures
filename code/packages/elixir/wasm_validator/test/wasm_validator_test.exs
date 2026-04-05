defmodule CodingAdventures.WasmValidatorTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.WasmValidator)
  end
end
