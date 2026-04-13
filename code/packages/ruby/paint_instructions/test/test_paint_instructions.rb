# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_paint_instructions"

class TestPaintInstructions < Minitest::Test
  def test_paint_rect
    rect = CodingAdventures::PaintInstructions.paint_rect(x: 1, y: 2, width: 3, height: 4)
    assert_equal "rect", rect.kind
    assert_equal 3, rect.width
  end

  def test_paint_scene
    scene = CodingAdventures::PaintInstructions.paint_scene(width: 10, height: 20, instructions: [])
    assert_equal 10, scene.width
    assert_equal 20, scene.height
  end
end
