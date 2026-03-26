defmodule CodingAdventures.DrawInstructionsSvgTest do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.DrawInstructionsSvg)
  end

  test "renders svg" do
    scene =
      CodingAdventures.DrawInstructions.create_scene(
        100,
        50,
        [CodingAdventures.DrawInstructions.draw_rect(10, 10, 20, 30)],
        "#ffffff",
        %{label: "demo"}
      )

    svg = CodingAdventures.DrawInstructionsSvg.render(scene)
    assert String.contains?(svg, "<svg")
    assert String.contains?(svg, ~s(aria-label="demo"))
  end

  test "renders text, groups, and escapes values" do
    group =
      CodingAdventures.DrawInstructions.draw_group(
        [CodingAdventures.DrawInstructions.draw_text(10, 20, "A&B", %{role: "label"})],
        %{layer: "labels"}
      )

    scene = CodingAdventures.DrawInstructions.create_scene(100, 50, [group])
    svg = CodingAdventures.DrawInstructionsSvg.render(scene)

    assert String.contains?(svg, "A&amp;B")
    assert String.contains?(svg, "<g")
    assert String.contains?(svg, ~s(data-layer="labels"))
  end
end
