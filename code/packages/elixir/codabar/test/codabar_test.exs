defmodule CodingAdventures.CodabarTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Codabar)
  end

  test "normalizes and draws codabar" do
    assert CodingAdventures.Codabar.normalize_codabar("40156") == "A40156A"

    scene = CodingAdventures.Codabar.draw_codabar("40156")
    assert scene.metadata.symbology == "codabar"
    assert scene.metadata.start == "A"
    assert scene.metadata.stop == "A"
    assert scene.width > 0
    assert scene.height == 120
  end

  test "expands inter-character gaps" do
    assert Enum.any?(
             CodingAdventures.Codabar.expand_codabar_runs("40156"),
             &(&1.role == "inter-character-gap")
           )
  end
end
