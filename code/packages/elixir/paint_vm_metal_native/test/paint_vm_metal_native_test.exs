defmodule CodingAdventures.PaintVmMetalNativeTest do
  use ExUnit.Case, async: false

  alias CodingAdventures.{PaintInstructions, PaintVmMetalNative, PixelContainer}

  test "renders a black rectangle on a white background through metal" do
    assert PaintVmMetalNative.available?()

    scene =
      PaintInstructions.paint_scene(
        40,
        20,
        [
          PaintInstructions.paint_rect(10, 0, 20, 20, "#000000")
        ],
        "#ffffff"
      )

    assert {:ok, %PixelContainer{} = pixels} = PaintVmMetalNative.render(scene)
    assert pixels.width == 40
    assert pixels.height == 20
    assert PixelContainer.pixel_at(pixels, 5, 10) == {255, 255, 255, 255}
    assert PixelContainer.pixel_at(pixels, 20, 10) == {0, 0, 0, 255}
  end
end
