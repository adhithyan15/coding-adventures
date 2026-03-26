# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_code39"

class TestCode39 < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Code39::VERSION
  end

  def test_encode_char
    encoded = CodingAdventures::Code39.encode_code39_char("A")
    assert_equal "WNNNNWNNW", encoded[:pattern]
  end

  def test_draw_scene
    scene = CodingAdventures::Code39.draw_code39("A")
    assert_equal "code39", scene.metadata[:symbology]
    assert_operator scene.width, :>, 0
  end
end
