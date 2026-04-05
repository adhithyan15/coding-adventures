defmodule CodingAdventures.WasmExecutionTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.WasmExecution)
  end
end
