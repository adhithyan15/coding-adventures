defmodule CodingAdventures.WasmRuntimeTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.WasmRuntime)
  end
end
