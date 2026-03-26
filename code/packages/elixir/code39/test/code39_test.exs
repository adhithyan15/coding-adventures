defmodule CodingAdventures.Code39Test do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Code39)
  end

  test "encodes and draws" do
    assert CodingAdventures.Code39.encode_code39_char("A").pattern == "WNNNNWNNW"

    scene = CodingAdventures.Code39.draw_code39("A")
    assert scene.metadata.symbology == "code39"
    assert scene.width > 0
  end
end
