defmodule CodingAdventures.PaintVmAsciiTest do
  use ExUnit.Case

  alias CodingAdventures.PaintInstructions
  alias CodingAdventures.PaintVmAscii
  alias CodingAdventures.PaintVmAscii.Version

  test "module exposes a version" do
    assert Version.version() == "0.1.0"
  end

  test "renders a filled rect with block characters" do
    scene =
      PaintInstructions.paint_scene(3, 2, [
        PaintInstructions.paint_rect(0, 0, 2, 1, "#000000")
      ])

    assert PaintVmAscii.render(scene, scale_x: 1, scale_y: 1) =~ "█"
  end

  test "transparent rect is invisible" do
    scene =
      PaintInstructions.paint_scene(3, 2, [
        PaintInstructions.paint_rect(0, 0, 2, 1, "transparent")
      ])

    assert PaintVmAscii.render(scene, scale_x: 1, scale_y: 1) == ""
  end
end
