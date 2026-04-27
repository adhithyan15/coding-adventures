defmodule CodingAdventures.BarcodeLayout1DTest do
  use ExUnit.Case

  test "builds runs from a binary pattern" do
    runs = CodingAdventures.BarcodeLayout1D.runs_from_binary_pattern("111001")

    assert Enum.map(runs, & &1.color) == ["bar", "space", "bar"]
    assert Enum.map(runs, & &1.modules) == [3, 2, 1]
  end

  test "lays out a paint scene" do
    runs =
      CodingAdventures.BarcodeLayout1D.runs_from_width_pattern(
        "WNW",
        ["bar", "space", "bar"],
        source_char: "A",
        source_index: 0
      )

    scene = CodingAdventures.BarcodeLayout1D.layout_barcode_1d(runs)

    assert scene.width == 27 * 4
    assert scene.height == 120
    assert length(scene.instructions) == 2
  end
end
