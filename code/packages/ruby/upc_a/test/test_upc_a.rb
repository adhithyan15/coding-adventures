# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_upc_a"

class TestUpcA < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::UpcA::VERSION
  end

  def test_compute_check_digit_matches_reference
    assert_equal "2", CodingAdventures::UpcA.compute_upc_a_check_digit("03600029145")
  end

  def test_normalize_appends_check_digit
    assert_equal "036000291452", CodingAdventures::UpcA.normalize_upc_a("03600029145")
  end

  def test_expand_runs_total_95_modules
    assert_equal 95, CodingAdventures::UpcA.expand_upc_a_runs("036000291452").sum { |run| run[:modules] }
  end

  def test_paint_scene
    scene = CodingAdventures::UpcA.draw_upc_a("03600029145")
    assert_equal "upc-a", scene.metadata[:symbology]
    assert_equal 95, scene.metadata[:content_modules]
    assert_operator scene.width, :>, 0
    assert_equal 120, scene.height
  end
end
