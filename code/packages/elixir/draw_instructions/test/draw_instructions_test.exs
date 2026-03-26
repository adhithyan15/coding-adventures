defmodule CodingAdventures.DrawInstructionsTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.DrawInstructions)
  end

  test "helpers build a scene" do
    rect = CodingAdventures.DrawInstructions.draw_rect(1, 2, 3, 4, "#111111", %{kind: "demo"})
    assert rect.kind == :rect
    assert rect.metadata.kind == "demo"

    scene = CodingAdventures.DrawInstructions.create_scene(100, 50, [rect])
    assert scene.background == "#ffffff"
  end

  test "text group and render_with helpers work" do
    text = CodingAdventures.DrawInstructions.draw_text(10, 20, "hello", %{role: "label"})
    assert text.kind == :text
    assert text.font_family == "monospace"
    assert text.fill == "#000000"

    group = CodingAdventures.DrawInstructions.draw_group([text], %{layer: "labels"})
    assert group.kind == :group

    renderer = %{render: fn scene -> "#{scene.width}x#{scene.height}" end}

    result =
      CodingAdventures.DrawInstructions.render_with(
        CodingAdventures.DrawInstructions.create_scene(10, 20, [group]),
        renderer
      )

    assert result == "10x20"
  end

  defmodule TestRenderer do
    def render(scene), do: "module:#{scene.width}x#{scene.height}"
  end

  test "render_with supports module renderers and defaults" do
    rect = CodingAdventures.DrawInstructions.draw_rect(0, 0, 5, 6)
    assert rect.fill == "#000000"

    scene =
      CodingAdventures.DrawInstructions.create_scene(7, 8, [rect], "#eeeeee", %{kind: "demo"})

    assert scene.metadata.kind == "demo"
    assert CodingAdventures.DrawInstructions.render_with(scene, TestRenderer) == "module:7x8"
  end

  test "defaults and invalid renderer branch" do
    group = CodingAdventures.DrawInstructions.draw_group([])
    assert group.metadata == %{}

    scene = CodingAdventures.DrawInstructions.create_scene(3, 4, [group])
    assert scene.background == "#ffffff"

    assert_raise ArgumentError, fn ->
      CodingAdventures.DrawInstructions.render_with(scene, %{})
    end
  end
end
