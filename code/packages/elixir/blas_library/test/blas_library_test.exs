defmodule CodingAdventures.BlasLibraryTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.BlasLibrary)
  end
end
