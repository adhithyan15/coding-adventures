# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_draw_instructions"

class TestDrawInstructions < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::DrawInstructions::VERSION
  end

  def test_rect_and_scene_helpers
    rect = CodingAdventures::DrawInstructions.draw_rect(x: 1, y: 2, width: 3, height: 4, metadata: { kind: "demo" })
    assert_equal "rect", rect.kind
    assert_equal "demo", rect.metadata[:kind]

    scene = CodingAdventures::DrawInstructions.create_scene(width: 100, height: 50, instructions: [rect])
    assert_equal "#ffffff", scene.background
  end

  def test_render_with_delegates
    scene = CodingAdventures::DrawInstructions.create_scene(width: 10, height: 10, instructions: [])
    renderer = Struct.new(:value) do
      def render(_scene)
        value
      end
    end.new("ok")

    assert_equal "ok", CodingAdventures::DrawInstructions.render_with(scene, renderer)
  end
end
