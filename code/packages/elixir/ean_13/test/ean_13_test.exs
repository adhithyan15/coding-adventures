defmodule CodingAdventures.Ean13Test do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Ean13)
  end

  test "computes check digit and parity" do
    assert CodingAdventures.Ean13.compute_ean_13_check_digit("400638133393") == "1"
    assert CodingAdventures.Ean13.left_parity_pattern("4006381333931") == "LGLLGG"
  end

  test "draws ean-13" do
    scene = CodingAdventures.Ean13.draw_ean_13("400638133393")
    assert scene.metadata.symbology == "ean-13"
    assert scene.metadata.content_modules == 95
    assert scene.width > 0
    assert scene.height == 120
  end
end
