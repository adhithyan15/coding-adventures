# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_codabar"

class TestCodabar < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Codabar::VERSION
  end

  def test_normalize_codabar_adds_default_guards
    assert_equal "A40156A", CodingAdventures::Codabar.normalize_codabar("40156")
  end

  def test_normalize_codabar_preserves_explicit_guards
    assert_equal "B1234D", CodingAdventures::Codabar.normalize_codabar("B1234D")
  end

  def test_expand_runs_marks_inter_character_gap
    runs = CodingAdventures::Codabar.expand_codabar_runs("40156")
    assert_includes runs.map { |run| run[:role] }, "inter-character-gap"
  end

  def test_paint_scene
    scene = CodingAdventures::Codabar.draw_codabar("40156")
    assert_equal "codabar", scene.metadata[:symbology]
    assert_equal "A", scene.metadata[:start]
    assert_equal "A", scene.metadata[:stop]
    assert_operator scene.width, :>, 0
    assert_equal 120, scene.height
  end
end
