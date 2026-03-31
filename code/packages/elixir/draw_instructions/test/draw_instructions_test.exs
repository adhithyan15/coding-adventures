defmodule CodingAdventures.DrawInstructionsTest do
  use ExUnit.Case

  alias CodingAdventures.DrawInstructions

  # ---------------------------------------------------------------------------
  # draw_rect
  # ---------------------------------------------------------------------------

  test "draw_rect creates correct struct with defaults" do
    rect = DrawInstructions.draw_rect(1, 2, 30, 40)

    assert rect.kind == :rect
    assert rect.x == 1
    assert rect.y == 2
    assert rect.width == 30
    assert rect.height == 40
    assert rect.fill == "#000000"
    assert rect.stroke == nil
    assert rect.stroke_width == nil
    assert rect.metadata == %{}
  end

  test "draw_rect with custom fill" do
    rect = DrawInstructions.draw_rect(0, 0, 10, 20, "#ff0000")
    assert rect.fill == "#ff0000"
  end

  test "draw_rect with stroke options" do
    rect =
      DrawInstructions.draw_rect(5, 5, 50, 30, "#ffffff",
        stroke: "#000000",
        stroke_width: 2,
        metadata: %{role: "border"}
      )

    assert rect.stroke == "#000000"
    assert rect.stroke_width == 2
    assert rect.metadata == %{role: "border"}
  end

  test "draw_rect with metadata only" do
    rect = DrawInstructions.draw_rect(0, 0, 10, 10, "#000000", metadata: %{kind: "demo"})
    assert rect.metadata.kind == "demo"
  end

  test "draw_rect backward compat: map as 6th arg treated as metadata" do
    rect = DrawInstructions.draw_rect(1, 2, 3, 4, "#111111", %{char: "A", index: 0})
    assert rect.kind == :rect
    assert rect.metadata == %{char: "A", index: 0}
    assert rect.stroke == nil
    assert rect.stroke_width == nil
  end

  # ---------------------------------------------------------------------------
  # draw_text
  # ---------------------------------------------------------------------------

  test "draw_text creates correct struct with defaults" do
    text = DrawInstructions.draw_text(10, 20, "hello")

    assert text.kind == :text
    assert text.x == 10
    assert text.y == 20
    assert text.value == "hello"
    assert text.fill == "#000000"
    assert text.font_family == "monospace"
    assert text.font_size == 16
    assert text.align == "middle"
    assert text.font_weight == nil
    assert text.metadata == %{}
  end

  test "draw_text with custom options" do
    text =
      DrawInstructions.draw_text(5, 10, "world",
        fill: "#ff0000",
        font_family: "Arial",
        font_size: 24,
        align: "start",
        metadata: %{role: "label"}
      )

    assert text.fill == "#ff0000"
    assert text.font_family == "Arial"
    assert text.font_size == 24
    assert text.align == "start"
    assert text.metadata == %{role: "label"}
  end

  test "draw_text with font_weight" do
    text = DrawInstructions.draw_text(0, 0, "bold text", font_weight: "bold")
    assert text.font_weight == "bold"
  end

  test "draw_text backward compat: map as 4th arg treated as metadata" do
    text = DrawInstructions.draw_text(10, 20, "hello", %{role: "label"})
    assert text.kind == :text
    assert text.metadata == %{role: "label"}
    assert text.font_family == "monospace"
    assert text.font_weight == nil
  end

  # ---------------------------------------------------------------------------
  # draw_group
  # ---------------------------------------------------------------------------

  test "draw_group creates struct with children" do
    rect = DrawInstructions.draw_rect(0, 0, 10, 10)
    text = DrawInstructions.draw_text(5, 5, "hi")
    group = DrawInstructions.draw_group([rect, text])

    assert group.kind == :group
    assert length(group.children) == 2
    assert group.metadata == %{}
  end

  test "draw_group with metadata" do
    group = DrawInstructions.draw_group([], %{layer: "labels"})
    assert group.metadata == %{layer: "labels"}
  end

  # ---------------------------------------------------------------------------
  # draw_line
  # ---------------------------------------------------------------------------

  test "draw_line creates correct struct with defaults" do
    line = DrawInstructions.draw_line(0, 0, 100, 0)

    assert line.kind == :line
    assert line.x1 == 0
    assert line.y1 == 0
    assert line.x2 == 100
    assert line.y2 == 0
    assert line.stroke == "#000000"
    assert line.stroke_width == 1
    assert line.metadata == %{}
  end

  test "draw_line with custom stroke and width" do
    line = DrawInstructions.draw_line(10, 20, 30, 40, "#ff0000", 3, %{role: "grid"})

    assert line.stroke == "#ff0000"
    assert line.stroke_width == 3
    assert line.metadata == %{role: "grid"}
  end

  # ---------------------------------------------------------------------------
  # draw_clip
  # ---------------------------------------------------------------------------

  test "draw_clip creates correct struct" do
    text = DrawInstructions.draw_text(5, 10, "clipped")
    clip = DrawInstructions.draw_clip(0, 0, 50, 20, [text])

    assert clip.kind == :clip
    assert clip.x == 0
    assert clip.y == 0
    assert clip.width == 50
    assert clip.height == 20
    assert length(clip.children) == 1
    assert clip.metadata == %{}
  end

  test "draw_clip with metadata" do
    clip = DrawInstructions.draw_clip(10, 10, 100, 50, [], %{cell: "A1"})
    assert clip.metadata == %{cell: "A1"}
  end

  # ---------------------------------------------------------------------------
  # create_scene
  # ---------------------------------------------------------------------------

  test "create_scene with defaults" do
    scene = DrawInstructions.create_scene(100, 50, [])

    assert scene.width == 100
    assert scene.height == 50
    assert scene.background == "#ffffff"
    assert scene.instructions == []
    assert scene.metadata == %{}
  end

  test "create_scene with custom background and metadata" do
    rect = DrawInstructions.draw_rect(0, 0, 10, 10)

    scene =
      DrawInstructions.create_scene(200, 100, [rect], "#eeeeee", %{label: "test scene"})

    assert scene.background == "#eeeeee"
    assert scene.metadata == %{label: "test scene"}
    assert length(scene.instructions) == 1
  end

  # ---------------------------------------------------------------------------
  # render_with
  # ---------------------------------------------------------------------------

  defmodule TestRenderer do
    @moduledoc false
    def render(scene), do: "module:#{scene.width}x#{scene.height}"
  end

  test "render_with delegates to module renderer" do
    scene = DrawInstructions.create_scene(7, 8, [])
    assert DrawInstructions.render_with(scene, TestRenderer) == "module:7x8"
  end

  test "render_with delegates to map renderer" do
    renderer = %{render: fn scene -> "#{scene.width}x#{scene.height}" end}
    scene = DrawInstructions.create_scene(10, 20, [])
    assert DrawInstructions.render_with(scene, renderer) == "10x20"
  end

  test "render_with raises for invalid map renderer" do
    scene = DrawInstructions.create_scene(3, 4, [])

    assert_raise ArgumentError, fn ->
      DrawInstructions.render_with(scene, %{})
    end
  end

  test "render_with raises for non-module non-map renderer" do
    scene = DrawInstructions.create_scene(3, 4, [])

    assert_raise ArgumentError, fn ->
      DrawInstructions.render_with(scene, "not a renderer")
    end
  end

  # ---------------------------------------------------------------------------
  # Integration: scene with all instruction types
  # ---------------------------------------------------------------------------

  test "scene with all instruction types" do
    rect = DrawInstructions.draw_rect(0, 0, 100, 50, "#cccccc", stroke: "#000000")
    text = DrawInstructions.draw_text(50, 25, "Title", font_weight: "bold")
    line = DrawInstructions.draw_line(0, 50, 100, 50, "#999999", 1)
    clip = DrawInstructions.draw_clip(10, 10, 80, 30, [text])
    group = DrawInstructions.draw_group([rect, line, clip], %{layer: "main"})
    scene = DrawInstructions.create_scene(100, 100, [group])

    assert scene.width == 100
    assert length(scene.instructions) == 1

    [g] = scene.instructions
    assert g.kind == :group
    assert length(g.children) == 3
  end
end
