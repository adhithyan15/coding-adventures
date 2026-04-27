defmodule CodingAdventures.ItfTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Itf)
  end

  test "encodes and draws" do
    encoded = CodingAdventures.Itf.encode_itf("123456")
    assert length(encoded) == 3
    assert hd(encoded).pair == "12"

    scene = CodingAdventures.Itf.draw_itf("123456")
    assert scene.metadata.symbology == "itf"
    assert scene.metadata.pair_count == 3
    assert scene.width > 0
    assert scene.height == 120
  end

  test "expands start and stop runs" do
    roles = Enum.map(CodingAdventures.Itf.expand_itf_runs("123456"), & &1.role)
    assert "start" in roles
    assert "stop" in roles
  end
end
