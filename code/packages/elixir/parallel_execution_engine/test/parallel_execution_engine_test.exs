defmodule CodingAdventures.ParallelExecutionEngineTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.ParallelExecutionEngine)
  end
end
