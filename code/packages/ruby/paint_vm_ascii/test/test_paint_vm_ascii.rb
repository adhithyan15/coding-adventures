# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_paint_vm_ascii"

class TestPaintVmAscii < Minitest::Test
  PI = CodingAdventures::PaintInstructions
  PVA = CodingAdventures::PaintVmAscii

  def test_version_exists
    assert_equal "0.1.0", PVA::VERSION
  end

  def test_filled_rect_renders_block_characters
    scene = PI.create_scene(width: 3, height: 2, instructions: [
      PI.paint_rect(x: 0, y: 0, width: 2, height: 1, fill: "#000000")
    ])

    assert_includes PVA.render(scene, scale_x: 1, scale_y: 1), "\u2588"
  end

  def test_transparent_rect_is_invisible
    scene = PI.create_scene(width: 3, height: 2, instructions: [
      PI.paint_rect(x: 0, y: 0, width: 2, height: 1, fill: "transparent")
    ])

    assert_equal "", PVA.render(scene, scale_x: 1, scale_y: 1)
  end
end
