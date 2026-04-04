defmodule CodingAdventures.MosaicAnalyzerTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.MosaicAnalyzer)
  end
end
