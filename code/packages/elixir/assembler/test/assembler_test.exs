defmodule CodingAdventures.AssemblerTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Assembler)
  end
end
