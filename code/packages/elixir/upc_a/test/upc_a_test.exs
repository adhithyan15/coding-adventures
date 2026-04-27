defmodule CodingAdventures.UpcATest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.UpcA)
  end

  test "computes check digit and draws" do
    assert CodingAdventures.UpcA.compute_upc_a_check_digit("03600029145") == "2"

    scene = CodingAdventures.UpcA.draw_upc_a("03600029145")
    assert scene.metadata.symbology == "upc-a"
    assert scene.metadata.content_modules == 95
    assert scene.width > 0
    assert scene.height == 120
  end
end
