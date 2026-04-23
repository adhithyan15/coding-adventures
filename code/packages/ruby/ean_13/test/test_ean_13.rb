# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_ean_13"

class TestEan13 < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Ean13::VERSION
  end

  def test_compute_check_digit_matches_reference
    assert_equal "1", CodingAdventures::Ean13.compute_ean_13_check_digit("400638133393")
  end

  def test_normalize_appends_check_digit
    assert_equal "4006381333931", CodingAdventures::Ean13.normalize_ean_13("400638133393")
  end

  def test_left_parity_pattern_matches_reference
    assert_equal "LGLLGG", CodingAdventures::Ean13.left_parity_pattern("4006381333931")
  end

  def test_expand_runs_total_95_modules
    assert_equal 95, CodingAdventures::Ean13.expand_ean_13_runs("4006381333931").sum { |run| run[:modules] }
  end

  def test_paint_scene
    scene = CodingAdventures::Ean13.draw_ean_13("400638133393")
    assert_equal "ean-13", scene.metadata[:symbology]
    assert_equal 95, scene.metadata[:content_modules]
    assert_operator scene.width, :>, 0
    assert_equal 120, scene.height
  end
end
