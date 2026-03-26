# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_draw_instructions_svg"

class TestDrawInstructionsSvg < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::DrawInstructionsSvg::VERSION
  end

  def test_render_svg
    scene = CodingAdventures::DrawInstructions.create_scene(
      width: 100,
      height: 50,
      instructions: [CodingAdventures::DrawInstructions.draw_rect(x: 10, y: 10, width: 20, height: 30)],
      metadata: { label: "demo" }
    )
    svg = CodingAdventures::DrawInstructionsSvg.render_svg(scene)
    assert_includes svg, "<svg"
    assert_includes svg, 'aria-label="demo"'
  end
end
