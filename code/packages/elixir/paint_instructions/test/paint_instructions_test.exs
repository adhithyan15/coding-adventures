defmodule CodingAdventures.PaintInstructionsTest do
  use ExUnit.Case

  test "builds a paint rect and scene" do
    rect = CodingAdventures.PaintInstructions.paint_rect(1, 2, 3, 4)
    scene = CodingAdventures.PaintInstructions.paint_scene(10, 20, [rect])

    assert rect.kind == :rect
    assert scene.width == 10
    assert scene.height == 20
  end

  test "create_scene delegates to paint_scene" do
    scene =
      CodingAdventures.PaintInstructions.create_scene(5, 6, [], "#eeeeee", %{kind: "demo"})

    assert scene.background == "#eeeeee"
    assert scene.metadata.kind == "demo"
  end
end
