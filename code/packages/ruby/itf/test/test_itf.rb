# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_itf"

class TestItf < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Itf::VERSION
  end

  def test_normalize_rejects_odd_input
    assert_raises(ArgumentError) { CodingAdventures::Itf.normalize_itf("12345") }
  end

  def test_encode_interleaves_digit_pairs
    encoded = CodingAdventures::Itf.encode_itf("123456")
    assert_equal 3, encoded.length
    assert_equal "12", encoded.first[:pair]
  end

  def test_expand_runs_include_start_and_stop
    roles = CodingAdventures::Itf.expand_itf_runs("123456").map { |run| run[:role] }
    assert_includes roles, "start"
    assert_includes roles, "stop"
  end

  def test_paint_scene
    scene = CodingAdventures::Itf.draw_itf("123456")
    assert_equal "itf", scene.metadata[:symbology]
    assert_equal 3, scene.metadata[:pair_count]
    assert_operator scene.width, :>, 0
    assert_equal 120, scene.height
  end
end
