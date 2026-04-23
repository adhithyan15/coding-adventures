defmodule CodingAdventures.Code128Test do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Code128)
  end

  test "computes checksum and draws" do
    values =
      "Code 128"
      |> String.graphemes()
      |> Enum.map(&CodingAdventures.Code128.value_for_code128_b_char/1)

    assert CodingAdventures.Code128.compute_code128_checksum(values) == 64

    scene = CodingAdventures.Code128.draw_code128("Code 128")
    assert scene.metadata.symbology == "code128"
    assert scene.metadata.code_set == "B"
    assert scene.width > 0
    assert scene.height == 120
  end

  test "encodes start checksum and stop" do
    encoded = CodingAdventures.Code128.encode_code128_b("Code 128")
    assert hd(encoded).role == "start"
    assert Enum.at(encoded, -2).role == "check"
    assert List.last(encoded).role == "stop"
  end
end
