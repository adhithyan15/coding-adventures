defmodule CodingAdventures.PaintCodecPngNativeTest do
  use ExUnit.Case, async: false

  alias CodingAdventures.{PaintCodecPngNative, PixelContainer}

  test "encodes RGBA pixels to a PNG binary" do
    pixels = %PixelContainer{
      width: 1,
      height: 1,
      data: <<0, 0, 0, 255>>
    }

    assert PaintCodecPngNative.available?()
    assert {:ok, png} = PaintCodecPngNative.encode(pixels)
    assert binary_part(png, 0, 8) == <<137, 80, 78, 71, 13, 10, 26, 10>>
  end
end
