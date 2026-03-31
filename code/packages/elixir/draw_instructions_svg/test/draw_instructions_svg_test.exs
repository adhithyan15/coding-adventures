defmodule CodingAdventures.DrawInstructionsSvgTest do
  use ExUnit.Case

  alias CodingAdventures.DrawInstructions
  alias CodingAdventures.DrawInstructionsSvg

  # ---------------------------------------------------------------------------
  # Basic rendering
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(DrawInstructionsSvg)
  end

  test "renders empty scene with svg wrapper and background" do
    scene = DrawInstructions.create_scene(100, 50, [], "#ffffff", %{label: "empty"})
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, "<svg")
    assert String.contains?(svg, ~s(width="100"))
    assert String.contains?(svg, ~s(height="50"))
    assert String.contains?(svg, ~s(aria-label="empty"))
    assert String.contains?(svg, "</svg>")
  end

  test "uses default label when metadata has no label" do
    scene = DrawInstructions.create_scene(10, 10, [])
    svg = DrawInstructionsSvg.render(scene)
    assert String.contains?(svg, ~s(aria-label="draw instructions scene"))
  end

  # ---------------------------------------------------------------------------
  # Rect rendering
  # ---------------------------------------------------------------------------

  test "renders rect instruction" do
    rect = DrawInstructions.draw_rect(10, 20, 30, 40, "#ff0000")
    scene = DrawInstructions.create_scene(100, 100, [rect])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, ~s(<rect x="10" y="20" width="30" height="40" fill="#ff0000"))
  end

  test "renders rect with stroke" do
    rect = DrawInstructions.draw_rect(0, 0, 50, 50, "#ffffff", stroke: "#000000", stroke_width: 2)
    scene = DrawInstructions.create_scene(100, 100, [rect])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, ~s(stroke="#000000"))
    assert String.contains?(svg, ~s(stroke-width="2"))
  end

  test "renders rect with stroke defaults width to 1" do
    rect = DrawInstructions.draw_rect(0, 0, 50, 50, "#ffffff", stroke: "#333333")
    scene = DrawInstructions.create_scene(100, 100, [rect])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, ~s(stroke="#333333"))
    assert String.contains?(svg, ~s(stroke-width="1"))
  end

  test "renders rect without stroke when nil" do
    rect = DrawInstructions.draw_rect(0, 0, 50, 50)
    scene = DrawInstructions.create_scene(100, 100, [rect])
    svg = DrawInstructionsSvg.render(scene)

    refute String.contains?(svg, "stroke=")
  end

  test "renders rect with metadata" do
    rect = DrawInstructions.draw_rect(0, 0, 10, 10, "#000000", metadata: %{role: "bar"})
    scene = DrawInstructions.create_scene(100, 100, [rect])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, ~s(data-role="bar"))
  end

  # ---------------------------------------------------------------------------
  # Text rendering
  # ---------------------------------------------------------------------------

  test "renders text instruction" do
    text = DrawInstructions.draw_text(50, 25, "Hello")
    scene = DrawInstructions.create_scene(100, 50, [text])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, "<text")
    assert String.contains?(svg, ~s(x="50"))
    assert String.contains?(svg, ~s(y="25"))
    assert String.contains?(svg, ">Hello</text>")
  end

  test "renders text with font_weight bold" do
    text = DrawInstructions.draw_text(0, 0, "Bold", font_weight: "bold")
    scene = DrawInstructions.create_scene(100, 50, [text])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, ~s(font-weight="bold"))
  end

  test "does not render font-weight when nil" do
    text = DrawInstructions.draw_text(0, 0, "Normal")
    scene = DrawInstructions.create_scene(100, 50, [text])
    svg = DrawInstructionsSvg.render(scene)

    refute String.contains?(svg, "font-weight")
  end

  test "does not render font-weight when normal" do
    text = DrawInstructions.draw_text(0, 0, "Normal", font_weight: "normal")
    scene = DrawInstructions.create_scene(100, 50, [text])
    svg = DrawInstructionsSvg.render(scene)

    refute String.contains?(svg, "font-weight")
  end

  test "escapes text content" do
    text = DrawInstructions.draw_text(0, 0, "A&B<C>")
    scene = DrawInstructions.create_scene(100, 50, [text])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, "A&amp;B&lt;C&gt;")
  end

  # ---------------------------------------------------------------------------
  # Line rendering
  # ---------------------------------------------------------------------------

  test "renders line instruction" do
    line = DrawInstructions.draw_line(0, 50, 100, 50, "#999999", 2)
    scene = DrawInstructions.create_scene(100, 100, [line])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, "<line")
    assert String.contains?(svg, ~s(x1="0"))
    assert String.contains?(svg, ~s(y1="50"))
    assert String.contains?(svg, ~s(x2="100"))
    assert String.contains?(svg, ~s(y2="50"))
    assert String.contains?(svg, ~s(stroke="#999999"))
    assert String.contains?(svg, ~s(stroke-width="2"))
  end

  test "renders line with metadata" do
    line = DrawInstructions.draw_line(0, 0, 10, 10, "#000000", 1, %{role: "grid"})
    scene = DrawInstructions.create_scene(100, 100, [line])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, ~s(data-role="grid"))
  end

  # ---------------------------------------------------------------------------
  # Group rendering
  # ---------------------------------------------------------------------------

  test "renders group with children" do
    rect = DrawInstructions.draw_rect(0, 0, 10, 10)
    text = DrawInstructions.draw_text(5, 5, "hi")
    group = DrawInstructions.draw_group([rect, text], %{layer: "labels"})
    scene = DrawInstructions.create_scene(100, 100, [group])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, "<g")
    assert String.contains?(svg, "</g>")
    assert String.contains?(svg, ~s(data-layer="labels"))
    assert String.contains?(svg, "<rect")
    assert String.contains?(svg, "<text")
  end

  # ---------------------------------------------------------------------------
  # Clip rendering
  # ---------------------------------------------------------------------------

  test "renders clip instruction with clipPath" do
    text = DrawInstructions.draw_text(5, 15, "clipped text")
    clip = DrawInstructions.draw_clip(0, 0, 50, 20, [text])
    scene = DrawInstructions.create_scene(100, 100, [clip])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, "<defs>")
    assert String.contains?(svg, "<clipPath")
    assert String.contains?(svg, ~s(id="clip-1"))
    assert String.contains?(svg, ~s[clip-path="url(#clip-1)"])
    assert String.contains?(svg, "clipped text")
    assert String.contains?(svg, "</clipPath>")
    assert String.contains?(svg, "</defs>")
  end

  test "clip IDs increment for multiple clips" do
    clip1 = DrawInstructions.draw_clip(0, 0, 50, 20, [])
    clip2 = DrawInstructions.draw_clip(50, 0, 50, 20, [])
    scene = DrawInstructions.create_scene(100, 100, [clip1, clip2])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, ~s(id="clip-1"))
    assert String.contains?(svg, ~s(id="clip-2"))
  end

  test "clip IDs reset between renders" do
    clip = DrawInstructions.draw_clip(0, 0, 50, 20, [])
    scene = DrawInstructions.create_scene(100, 100, [clip])

    svg1 = DrawInstructionsSvg.render(scene)
    svg2 = DrawInstructionsSvg.render(scene)

    # Both renders should produce clip-1, not clip-1 then clip-2
    assert String.contains?(svg1, ~s(id="clip-1"))
    assert String.contains?(svg2, ~s(id="clip-1"))
  end

  test "renders clip with metadata" do
    clip = DrawInstructions.draw_clip(0, 0, 50, 20, [], %{cell: "A1"})
    scene = DrawInstructions.create_scene(100, 100, [clip])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, ~s(data-cell="A1"))
  end

  # ---------------------------------------------------------------------------
  # Integration
  # ---------------------------------------------------------------------------

  test "renders scene with all instruction types" do
    rect = DrawInstructions.draw_rect(0, 0, 100, 50, "#cccccc", stroke: "#000000")
    text = DrawInstructions.draw_text(50, 25, "Title", font_weight: "bold")
    line = DrawInstructions.draw_line(0, 50, 100, 50, "#999999")
    clip = DrawInstructions.draw_clip(10, 10, 80, 30, [text])
    group = DrawInstructions.draw_group([rect, line], %{layer: "bg"})

    scene = DrawInstructions.create_scene(100, 100, [group, clip], "#ffffff", %{label: "full"})
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, "<svg")
    assert String.contains?(svg, "<rect")
    assert String.contains?(svg, "<text")
    assert String.contains?(svg, "<line")
    assert String.contains?(svg, "<clipPath")
    assert String.contains?(svg, "<g")
    assert String.contains?(svg, "</svg>")
  end

  test "render_with integration via DrawInstructions" do
    scene = DrawInstructions.create_scene(10, 20, [])
    svg = DrawInstructions.render_with(scene, DrawInstructionsSvg)

    assert String.contains?(svg, "<svg")
    assert String.contains?(svg, ~s(width="10"))
  end

  # ---------------------------------------------------------------------------
  # XML escaping
  # ---------------------------------------------------------------------------

  test "escapes special characters in metadata values" do
    rect = DrawInstructions.draw_rect(0, 0, 10, 10, "#000000", metadata: %{note: "a&b\"c'd"})
    scene = DrawInstructions.create_scene(100, 100, [rect])
    svg = DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, "a&amp;b&quot;c&apos;d")
  end
end
